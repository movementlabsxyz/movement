use super::metadata::Metadata;
use crate::backend::s3::bucket_connection::BucketConnection;
use crate::backend::PushOperations;
use crate::files::package::{Package, PackageElement};
use aws_sdk_s3::operation::put_object::PutObjectOutput;
use aws_sdk_s3::primitives::ByteStream;
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
}

impl Push {
	pub fn new(bucket_connection: BucketConnection, metadata: Metadata) -> Self {
		Self { bucket_connection, metadata }
	}

	pub(crate) async fn upload_path(
		&self,
		relative_path: &std::path::Path,
		full_path: &std::path::Path,
	) -> Result<(PutObjectOutput, PathBuf), anyhow::Error> {
		let bucket = self.bucket_connection.bucket.clone();
		let key =
			format!("{}/{}", self.metadata.syncer_epoch_prefix()?, relative_path.to_string_lossy());
		let body = ByteStream::from_path(full_path).await?;
		let s3_path = format!("s3://{}/{}", bucket, key);
		let output = self
			.bucket_connection
			.client
			.put_object()
			.bucket(bucket)
			.body(body)
			.key(&key)
			.send()
			.await?;
		Ok((output, s3_path.into()))
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
			let future = self.upload_path(&relative_path, &full_path);
			manifest_futures.push(future);
		}

		// try to join all the manifest_futures
		let put_object_outputs = futures::future::try_join_all(manifest_futures).await?;
		let mut new_manifest = PackageElement::new(self.bucket_connection.bucket.clone().into());
		for (_, s3_path) in put_object_outputs {
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
		// prune the old epochs
		self.prune().await?;

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
