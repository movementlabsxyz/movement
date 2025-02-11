use super::metadata::Metadata;
use crate::backend::s3::bucket_connection::BucketConnection;
use crate::backend::s3::shared_bucket::BUFFER_SIZE;
use crate::backend::PullOperations;
use crate::files::package::{Package, PackageElement};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::{Read, Write};
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
		tracing::info!("Pulling file from S3 on bucket:{bucket} with key: {key}");
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

		//filter all path with only the one that contains the complete upload marker file.
		let to_remove: Vec<_> = candidates
			.iter()
			.filter_map(|(key, set)| {
				if set
					.iter()
					.find(|path| path.ends_with(super::UPLOAD_COMPLETE_MARKER_FILE_NAME))
					.is_none()
				{
					Some(key.clone())
				} else {
					None
				}
			})
			.collect();
		to_remove.iter().for_each(|key| {
			candidates.remove(key);
		});

		Ok(candidates.keys().cloned().collect())
	}

	pub(crate) async fn download_all_files_for_candidate(
		&self,
		candidate: &Candidate,
	) -> Result<PackageElement, anyhow::Error> {
		tracing::debug!("Downloading all files for candidate: {:?}", candidate);

		// get all of the public file paths for this application
		let public_file_paths = self
			.metadata
			.list_all_application_file_paths_for(&self.bucket_connection)
			.await?;

		// Filter the public file paths for the candidate.
		// Use BTreeSet to order the file chunks.
		let file_paths: BTreeSet<String> = public_file_paths
			.into_iter()
			.filter(|file_path| file_path.starts_with(&candidate.key))
			.collect();

		// create a new manifest
		let mut manifest = PackageElement::new(self.pull_destination.clone());

		// download each file
		let mut manifest_futures = Vec::new();
		for file_path in file_paths {
			//remove complte marker file
			if file_path.ends_with(super::UPLOAD_COMPLETE_MARKER_FILE_NAME) {
				continue;
			}
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

		//recreate splited archive if needed.
		let mut unsplit_manifest =
			PackageElement { sync_files: vec![], root_dir: manifest.root_dir.clone() };
		for absolute_path in &manifest.sync_files {
			let path_buf = absolute_path.to_path_buf();
			let absolute_path =
				tokio::task::spawn_blocking(move || recreate_archive(path_buf)).await??;

			if !unsplit_manifest.sync_files.contains(&absolute_path) {
				tracing::info!("Archive file added {absolute_path:?}",);
				unsplit_manifest.sync_files.push(absolute_path)
			}
		}

		// should downloaded into the locations specified in the manifest
		Ok(unsplit_manifest)
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
		_package: Package,
		candidate: Candidate,
	) -> Result<Package, anyhow::Error> {
		// pull all of the files for the candidate
		let manifest = self.download_all_files_for_candidate(&candidate).await?;
		let manifests = vec![manifest];

		Ok(Package(manifests))
	}
}

#[async_trait::async_trait]
impl PullOperations for Pull {
	async fn pull(&self, package: Option<Package>) -> Result<Option<Package>, anyhow::Error> {
		tracing::debug!("S3 pulling package: {:?}", package);
		if package.is_none() {
			return Ok(None);
		}

		let package = package.ok_or(anyhow::anyhow!("package is none"))?;

		let candidates = self.find_candidates(&package).await?;

		if candidates.is_empty() {
			return Ok(None);
		}

		let candidate_selected = self.select_candidate_from(&package, candidates).await?;
		Ok(Some(self.pull_candidate(package, candidate_selected).await?))
	}
}

fn recreate_archive(archive_chunk: PathBuf) -> Result<PathBuf, anyhow::Error> {
	if archive_chunk.extension().map(|ext| ext != "chunk").unwrap_or(true) {
		//not a chunk file return.
		return Ok(archive_chunk);
	}

	let (chunk, archive_file_name) = archive_chunk
		.file_name()
		.and_then(|file_name| file_name.to_str())
		.and_then(|file_name_str| file_name_str.strip_suffix(".chunk"))
		.and_then(|base_filename| {
			let base_filename_parts: Vec<&str> = base_filename.rsplitn(2, '_').collect();
			(base_filename_parts.len() > 1)
				.then(|| (base_filename_parts[0].to_string(), base_filename_parts[1].to_string()))
		})
		.ok_or(anyhow::anyhow!(format!(
			"Archive filename not found for chunk path:{:?}",
			archive_chunk.to_str()
		)))?;

	let archive_path = archive_chunk.parent().map(|parent| parent.join(archive_file_name)).ok_or(
		anyhow::anyhow!(format!(
			"Archive filename no root dir in path:{:?}",
			archive_chunk.to_str()
		)),
	)?;

	//remove old archive file
	if chunk == "000" && archive_path.exists() {
		std::fs::remove_file(&archive_path)?;
	}

	let mut archive_file = OpenOptions::new()
		.create(true) // Create the file if it doesn't exist
		.append(true) // Open in append mode (do not overwrite)
		.open(&archive_path)?;

	let mut buffer = vec![0; BUFFER_SIZE];

	let chunk_file = File::open(&archive_chunk)?;
	let mut chunk_reader = BufReader::new(chunk_file);

	loop {
		// Read a part of the chunk into the buffer
		let bytes_read = chunk_reader.read(&mut buffer)?;

		if bytes_read == 0 {
			break; // End of chunk file
		}

		// Write the buffer data to the output file
		archive_file.write_all(&buffer[..bytes_read])?;
	}

	let file_metadata = std::fs::metadata(&archive_path)?;
	let file_size = file_metadata.len() as usize;

	//remove the chunk that is useless.
	std::fs::remove_file(&archive_chunk)?;
	tracing::debug!("PULL {archive_path:?} archive_chunk size: {file_size}",);

	Ok(archive_path)
}
