use super::metadata::Metadata;
use crate::backend::s3::bucket_connection::BucketConnection;
use crate::backend::s3::shared_bucket::execute_with_concurrency_limit;
use crate::backend::s3::shared_bucket::BUFFER_SIZE;
use crate::backend::s3::shared_bucket::DEFAULT_CHUNK_SIZE;
use crate::backend::PushOperations;
use crate::files::package::{Package, PackageElement};
use aws_sdk_s3::operation::put_object::PutObjectOutput;
use aws_sdk_s3::primitives::ByteStream;
use std::fs::File;
use std::io::{BufReader as StdBufReader, Read, Write};
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Candidate {
	pub key: String,
	pub sync_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct Push {
	pub bucket_connection: BucketConnection,
	pub metadata: Metadata,
	pub chunk_size: usize,
	pub buffer_size: usize,
}

impl Push {
	pub fn new(bucket_connection: BucketConnection, metadata: Metadata) -> Self {
		Self {
			bucket_connection,
			metadata,
			chunk_size: DEFAULT_CHUNK_SIZE,
			buffer_size: BUFFER_SIZE,
		}
	}

	pub(crate) async fn upload_path(
		bucket_connection: BucketConnection,
		syncer_epoch_prefix: String,
		relative_path: std::path::PathBuf,
		full_path: std::path::PathBuf,
	) -> Result<(PutObjectOutput, PathBuf), anyhow::Error> {
		let bucket = bucket_connection.bucket.clone();
		let key = format!("{}/{}", syncer_epoch_prefix, relative_path.to_string_lossy());
		tracing::info!("Pushing file on S3 on bucket:{bucket} with key: {key}");
		let body = ByteStream::from_path(full_path).await?;
		let s3_path = format!("s3://{}/{}", bucket, key);
		let output = bucket_connection
			.client
			.put_object()
			.bucket(bucket)
			.body(body)
			.key(&key)
			.send()
			.await?;
		Ok((output, s3_path.into()))
	}

	async fn add_marker_file(
		bucket_connection: BucketConnection,
		syncer_epoch_prefix: String,

		marker_name: &str,
	) -> Result<(PutObjectOutput, PathBuf), anyhow::Error> {
		let bucket = bucket_connection.bucket.clone();
		let marker_key = format!("{}/{}", syncer_epoch_prefix, marker_name);
		let s3_path = format!("s3://{}/{}", bucket, marker_key);
		let output = bucket_connection
			.client
			.put_object()
			.bucket(bucket)
			.key(marker_key)
			.body(ByteStream::from_static(b"Upload complete"))
			.send()
			.await?;
		Ok((output, s3_path.into()))
	}

	// Adapter method for the upload_path and add_marker_file future.
	async fn add_upload_entry(
		bucket_connection: BucketConnection,
		syncer_epoch_prefix: String,
		relative_path: std::path::PathBuf,
		full_path: std::path::PathBuf,
		marker_file: Option<&str>,
	) -> Result<(PutObjectOutput, PathBuf), anyhow::Error> {
		match marker_file {
			Some(file) => Push::add_marker_file(bucket_connection, syncer_epoch_prefix, file).await,
			None => {
				Push::upload_path(bucket_connection, syncer_epoch_prefix, relative_path, full_path)
					.await
			}
		}
	}

	pub(crate) async fn upload_based_on_manifest(
		&self,
		manifest: PackageElement,
	) -> Result<PackageElement, anyhow::Error> {
		// get the path tuples
		let path_tuples = manifest.try_path_tuples()?;

		// upload each file
		let mut manifest_futures = Vec::new();
		for (relative_path, full_path) in path_tuples {
			let future = Push::add_upload_entry(
				self.bucket_connection.clone(),
				self.metadata.syncer_epoch_prefix()?,
				relative_path,
				full_path,
				None,
			);
			manifest_futures.push(future);
		}

		// Add upload completed marker file
		let future = Push::add_upload_entry(
			self.bucket_connection.clone(),
			self.metadata.syncer_epoch_prefix()?,
			Default::default(),
			Default::default(),
			Some(super::UPLOAD_COMPLETE_MARKER_FILE_NAME),
		);
		manifest_futures.push(future);
		// Execute file upload with max 100 upload started at a time.
		let put_object_outputs = execute_with_concurrency_limit(manifest_futures, 100).await;
		let mut new_manifest = PackageElement::new(self.bucket_connection.bucket.clone().into());
		for res in put_object_outputs {
			let Ok((_, s3_path)) = res? else { todo!() };
			new_manifest.add_sync_file(s3_path);
		}

		Ok(new_manifest)
	}

	/// Prunes older epochs
	pub async fn prune(&self) -> Result<(), anyhow::Error> {
		// get all of the epochs for this application and syncer
		let public_sync_epochs = self
			.metadata
			.list_all_application_syncer_epochs(&self.bucket_connection)
			.await?;

		// sort them by the epoch (latest first)
		let mut sorted_sync_epochs: Vec<_> = public_sync_epochs.into_iter().collect();
		sorted_sync_epochs.sort_by_key(|epoch| -(*epoch as i64));

		// keep the first retain_epochs_count epochs
		let epochs_to_delete: Vec<_> = sorted_sync_epochs
			.into_iter()
			.skip(self.metadata.retain_epochs_count as usize)
			.collect();

		// delete the epochs
		for epoch in epochs_to_delete {
			let prefix = format!("{}/{}", self.metadata.syncer_prefix()?, epoch);
			let mut continuation_token = None;
			loop {
				let list_objects_output = self
					.bucket_connection
					.client
					.list_objects_v2()
					.bucket(self.bucket_connection.bucket.clone())
					.prefix(&prefix)
					.set_continuation_token(continuation_token)
					.send()
					.await?;
				if let Some(contents) = list_objects_output.contents {
					for object in contents {
						if let Some(key) = object.key {
							self.bucket_connection
								.client
								.delete_object()
								.bucket(self.bucket_connection.bucket.clone())
								.key(&key)
								.send()
								.await?;
						}
					}
				}
				if let Some(token) = list_objects_output.next_continuation_token {
					continuation_token = Some(token);
				} else {
					break;
				}
			}
		}

		Ok(())
	}
}

#[async_trait::async_trait]
impl PushOperations for Push {
	async fn push(&self, package: Package) -> Result<Package, anyhow::Error> {
		tracing::debug!("Pushing package:{package:?}");
		// prune the old epochs
		self.prune().await?;

		// Split the too big files
		let mut new_package_elements = vec![];
		for element in package.0 {
			for file_path in element.sync_files {
				let new_files =
					split_archive(file_path, &element.root_dir, self.chunk_size, self.buffer_size)?;
				let mut new_element = PackageElement::new(element.root_dir.clone());
				for dest in new_files {
					new_element.add_sync_file(dest);
				}
				new_package_elements.push(new_element);
			}
		}
		let package = Package(new_package_elements);

		// upload the package
		let mut manifest_futures = Vec::new();
		for manifest in package.into_manifests() {
			let future = self.upload_based_on_manifest(manifest);
			manifest_futures.push(future);
		}
		let manifests = futures::future::try_join_all(manifest_futures).await?;
		Ok(Package(manifests))
	}
}

fn split_archive<P: AsRef<Path>>(
	archive: PathBuf,
	root_dir: P,
	chunk_size: usize,
	buffer_size: usize,
) -> Result<Vec<PathBuf>, anyhow::Error> {
	let output_dir = root_dir.as_ref();

	// Check the file size before proceeding with the split
	let file_metadata = std::fs::metadata(&archive)?;
	let file_size = file_metadata.len() as usize;
	if file_size <= chunk_size {
		return Ok(vec![archive]);
	}

	let archive_file = File::open(&archive)?;

	std::fs::create_dir_all(output_dir)?;

	let mut chunk_num = 0;
	let mut buffer = vec![0; buffer_size];

	let archive_relative_path = archive.strip_prefix(&output_dir)?;
	let mut input_reader = StdBufReader::new(archive_file);

	let mut chunk_list = vec![];
	loop {
		// Create a new file for the chunk
		let chunk_path = output_dir.join(format!(
			"{}_{:03}.chunk",
			archive_relative_path.to_string_lossy(),
			chunk_num
		));

		let mut chunk_file = File::create(&chunk_path)?;

		let mut all_read_bytes = 0;
		let end = loop {
			// Read a part of the chunk into the buffer
			let bytes_read = input_reader.read(&mut buffer)?;
			if bytes_read == 0 {
				break true; // End of chunk file
			}

			// Write the buffer data to the output file
			chunk_file.write_all(&buffer[..bytes_read])?;
			all_read_bytes += bytes_read;
			if all_read_bytes >= chunk_size {
				break false;
			}
		};

		if all_read_bytes == 0 {
			break; // End of chunk file and discard the current one.
		}

		chunk_num += 1;
		chunk_list.push(chunk_path);
		if end {
			break; // End of chunk file
		}
	}

	tracing::info!("split_archive return {chunk_list:?}",);
	Ok(chunk_list)
}
