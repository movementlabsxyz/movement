use super::metadata::Metadata;
use crate::backend::s3::bucket_connection::BucketConnection;
use crate::backend::PullOperations;
use crate::files::package::{Package, PackageElement};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Candidate {
	pub key: String,
	pub sync_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct Pull {
	pub bucket_connection: BucketConnection,
	pub metadata: Metadata,
}

impl Pull {
	pub fn new(bucket_connection: BucketConnection, metadata: Metadata) -> Self {
		Self { bucket_connection, metadata }
	}

	pub(crate) async fn download_path(
		&self,
		candidate_selected: &Candidate,
		relative_path: &std::path::Path,
		full_path: &std::path::Path,
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
		let mut file = tokio::fs::File::create(full_path).await?;
		while let Some(chunk) = output.body.try_next().await? {
			file.write_all(&chunk).await?;
		}
		Ok(full_path.into())
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

		// filter the candidates based on whether they have all the files in the package
		/*let mut filtered_candidates = HashSet::new();
		for (candidate, file_paths) in candidates {
			let mut missing_files = false;
			for manifest in package.0.iter() {
				for (relative_path, _) in manifest.try_path_tuples()? {
					let full_path =
						format!("{}/{}", candidate.key, relative_path.to_string_lossy());
					if !file_paths.contains(&full_path) {
						missing_files = true;
						break;
					}
				}
				if missing_files {
					break;
				}
			}
			if !missing_files {
				filtered_candidates.insert(candidate);
			}
		}*/

		Ok(candidates.keys().cloned().collect())
	}

	pub(crate) async fn download_based_on_manifest(
		&self,
		candidate_selected: &Candidate,
		manifest: PackageElement,
	) -> Result<PackageElement, anyhow::Error> {
		// get the path tuples
		let path_tuples = manifest.try_path_tuples()?;

		// download each file
		let mut manifest_futures = Vec::new();
		for (relative_path, full_path) in path_tuples {
			let future = self.download_path(candidate_selected, &relative_path, &full_path);
			manifest_futures.push(future);
		}

		// try to join all the manifest_futures
		futures::future::try_join_all(manifest_futures).await?;

		// should downloaded into the locations specified in the manifest
		Ok(manifest)
	}

	async fn find_candidates(&self, package: &Package) -> Result<Vec<Candidate>, anyhow::Error> {
		let candidates = self.candidates_for(package).await?;
		Ok(candidates.into_iter().collect())
	}

	async fn select_candidate_from(
		&self,
		_package: &Package,
		mut candidates: Vec<Candidate>,
	) -> Result<Candidate, anyhow::Error> {
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
		package: Package,
		candidate: Candidate,
	) -> Result<Package, anyhow::Error> {
		let mut manifest_futures = Vec::new();
		for manifest in package.into_manifests() {
			let future = self.download_based_on_manifest(&candidate, manifest);
			manifest_futures.push(future);
		}
		let manifests = futures::future::try_join_all(manifest_futures).await?;
		Ok(Package(manifests))
	}
}

#[async_trait::async_trait]
impl PullOperations for Pull {
	async fn pull(&self, package: Package) -> Result<Package, anyhow::Error> {
		let candidates = self.find_candidates(&package).await?;
		let candidate_selected = self.select_candidate_from(&package, candidates).await?;
		self.pull_candidate(package, candidate_selected).await
	}
}
