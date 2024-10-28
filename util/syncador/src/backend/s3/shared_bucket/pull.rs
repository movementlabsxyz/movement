use super::metadata::Metadata;
use crate::backend::s3::bucket_connection::BucketConnection;
use crate::backend::PullOperations;
use crate::files::package::{Package, PackageElement};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Candidate {
	pub key: String,
	pub sync_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct Pull {
	pub bucket_connection: BucketConnection,
	pub metadata: Metadata,
	pub pull_destination: PathBuf,
}

impl Pull {
	pub fn new(
		bucket_connection: BucketConnection,
		metadata: Metadata,
		pull_destination: PathBuf,
	) -> Self {
		Self { bucket_connection, metadata, pull_destination }
	}

	pub(crate) async fn download_path(
		&self,
		candidate_selected: &Candidate,
		relative_path: std::path::PathBuf,
		full_path: std::path::PathBuf,
	) -> Result<PathBuf, anyhow::Error> {
		let bucket = self.bucket_connection.bucket.clone();
		let key = format!("{}/{}", candidate_selected.key, relative_path.to_string_lossy());
		let mut output = self
			.bucket_connection
			.client
			.get_object()
			.bucket(bucket)
			.key(&key)
			.send()
			.await?;
		// make any of the parent directories that don't exist
		tokio::fs::create_dir_all(
			full_path
				.parent()
				.ok_or(anyhow::anyhow!("parent directory of file path does not exist"))?,
		)
		.await?;
		let mut file = tokio::fs::File::create(full_path.clone()).await?;
		while let Some(chunk) = output.body.try_next().await? {
			file.write_all(&chunk).await?;
		}
		Ok(full_path)
	}

	pub(crate) async fn candidates_for(
		&self,
		_package: &Package,
	) -> Result<HashSet<Candidate>, anyhow::Error> {
		// for the candidates hash map with the bucket object candidate as the key
		let mut candidates: HashMap<Candidate, HashSet<String>> = HashMap::new();

		// get all of the public file paths for this application
		let public_file_paths = self
			.metadata
			.list_all_application_file_paths_for(&self.bucket_connection)
			.await?;

		info!("Public file paths: {:?}", public_file_paths);
		for file_path in public_file_paths {
			// the first three parts are the candidate key
			let parts: Vec<&str> = file_path.split('/').into_iter().take(3).collect();

			// the third part is the sync_epoch
			let sync_epoch = parts[2].parse::<u64>()?;

			// the candidate key
			let candidate = Candidate { key: parts.join("/"), sync_epoch };

			// add the file path to the candidate
			candidates.entry(candidate).or_insert_with(HashSet::new).insert(file_path);
		}

		Ok(candidates.keys().cloned().collect())
	}

	/*pub(crate) async fn download_based_on_manifest(
		&self,
		candidate_selected: &Candidate,
		manifest: PackageElement,
	) -> Result<PackageElement, anyhow::Error> {
		// get the path tuples
		let path_tuples = manifest.try_path_tuples()?;

		// download each file
		let mut manifest_futures = Vec::new();
		for (relative_path, full_path) in path_tuples {
			let future = self.download_path(
				candidate_selected,
				relative_path.to_path_buf().clone(),
				full_path.clone(),
			);
			manifest_futures.push(future);
		}

		// try to join all the manifest_futures
		futures::future::try_join_all(manifest_futures).await?;

		// should downloaded into the locations specified in the manifest
		Ok(manifest)
	}*/

	pub(crate) async fn download_all_files_for_candidate(
		&self,
		candidate: &Candidate,
	) -> Result<PackageElement, anyhow::Error> {
		info!("Downloading all files for candidate: {:?}", candidate);

		// get all of the public file paths for this application
		let public_file_paths = self
			.metadata
			.list_all_application_file_paths_for(&self.bucket_connection)
			.await?;

		// filter the public file paths for the candidate
		let file_paths: HashSet<String> = public_file_paths
			.into_iter()
			.filter(|file_path| file_path.starts_with(&candidate.key))
			.collect();

		// create a new manifest
		let mut manifest = PackageElement::new(self.pull_destination.clone());

		// download each file
		let mut manifest_futures = Vec::new();
		for file_path in file_paths {
			let relative_path = PathBuf::from(
				file_path
					.strip_prefix(format!("{}/", candidate.key).as_str())
					.ok_or(anyhow::anyhow!("could not strip prefix"))?,
			);
			let full_path = self.pull_destination.join(&relative_path);
			let future = self.download_path(candidate, relative_path.clone(), full_path.clone());
			manifest_futures.push(future);
			manifest.add_sync_file(full_path);
		}

		// try to join all the manifest_futures
		futures::future::try_join_all(manifest_futures).await?;

		// should downloaded into the locations specified in the manifest
		Ok(manifest)
	}

	async fn find_candidates(&self, package: &Package) -> Result<Vec<Candidate>, anyhow::Error> {
		info!("Finding candidates for package: {:?}", package);
		let candidates = self.candidates_for(package).await?;
		Ok(candidates.into_iter().collect())
	}

	async fn select_candidate_from(
		&self,
		_package: &Package,
		mut candidates: Vec<Candidate>,
	) -> Result<Candidate, anyhow::Error> {
		info!("Selecting from candidates: {:?}", candidates);
		// sort the intersection of candidates by the epoch (latest first)
		candidates.sort_by_key(|candidate| -(candidate.sync_epoch as i64));

		// pick the first candidate, if there are none, return an error
		if let Some(candidate) = candidates.first() {
			Ok((*candidate).clone())
		} else {
			Err(anyhow::anyhow!("no candidate found"))
		}
	}

	async fn pull_candidate(
		&self,
		_package: Package,
		candidate: Candidate,
	) -> Result<Package, anyhow::Error> {
		info!("Pulling candidate: {:?}", candidate);
		// pull all of the files for the candidate
		let manifest = self.download_all_files_for_candidate(&candidate).await?;
		let manifests = vec![manifest];

		Ok(Package(manifests))
	}
}

#[async_trait::async_trait]
impl PullOperations for Pull {
	async fn pull(&self, package: Option<Package>) -> Result<Option<Package>, anyhow::Error> {
		info!("S3 pulling package: {:?}", package);
		if package.is_none() {
			return Ok(None);
		}

		let package = package.ok_or(anyhow::anyhow!("package is none"))?;

		let candidates = self.find_candidates(&package).await?;

		info!("Candidates: {:?}", candidates);
		if candidates.is_empty() {
			return Ok(None);
		}

		let candidate_selected = self.select_candidate_from(&package, candidates).await?;
		Ok(Some(self.pull_candidate(package, candidate_selected).await?))
	}
}
