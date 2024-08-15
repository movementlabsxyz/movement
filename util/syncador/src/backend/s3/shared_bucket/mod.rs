use super::bucket_connection;

pub mod metadata;
pub mod pull;
pub mod push;

pub async fn create_random(bucket: String) -> Result<(push::Push, pull::Pull), anyhow::Error> {
	let config = aws_config::load_from_env().await;
	let client = aws_sdk_s3::Client::new(&config);
	create(client, bucket, metadata::Metadata::random()).await
}

pub async fn create(
	client: aws_sdk_s3::Client,
	bucket: String,
	metadata: metadata::Metadata,
) -> Result<(push::Push, pull::Pull), anyhow::Error> {
	let bucket_connection = bucket_connection::BucketConnection::create(client, bucket).await?;

	let push = push::Push::new(bucket_connection.clone(), metadata.clone());
	let pull = pull::Pull::new(bucket_connection, metadata);

	Ok((push, pull))
}

#[cfg(test)]
pub mod test {
	//! pub in case we want to reuse helpers

	use super::*;
	use crate::backend::{PullOperations, PushOperations};
	use crate::files::package::{Package, PackageElement};

	#[tokio::test]
	async fn test_create() -> Result<(), anyhow::Error> {
		// use uuid to generate a random bucket identifier
		let bucket = format!("public-test-bucket-{}", uuid::Uuid::new_v4());
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_s3::Client::new(&config);
		let (_push, pull) =
			create(client.clone(), bucket.clone(), metadata::Metadata::random()).await?;

		// check that the buckets exist
		let bucket_exists = client.head_bucket().bucket(bucket.clone()).send().await.is_ok();
		assert!(bucket_exists);

		let pull::Pull { bucket_connection, metadata: _ } = pull;
		bucket_connection.destroy(false).await?;

		// check that the buckets don't exist
		let bucket_exists = client.head_bucket().bucket(bucket.clone()).send().await.is_ok();

		assert!(!bucket_exists);

		Ok(())
	}

	#[tokio::test]
	async fn test_upload_download_many() -> Result<(), anyhow::Error> {
		// use uuid to generate a random bucket identifier
		let bucket = format!("public-test-bucket-{}", uuid::Uuid::new_v4());
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_s3::Client::new(&config);
		let (push, pull) =
			create(client.clone(), bucket.clone(), metadata::Metadata::random()).await?;

		// create a tempdir
		let tempdir = tempfile::tempdir()?;
		let root_dir = tempdir.path().to_path_buf();

		// create many tempfiles with varying path nesting
		// use a modulus of the index to determine the nesting level
		// use a different modulus to select the subdirs
		let mut paths = Vec::new();
		for i in 0..100 {
			let mut path = root_dir.clone();
			let mut index = i;
			for _ in 0..(i % 5) {
				let subdir = format!("subdir{}", index % 3);
				path.push(subdir);
				index /= 3;
			}

			path.push(format!("file{}.txt", i));

			// make all of the parents of the file path, if they don't exist
			tokio::fs::create_dir_all(
				path.parent()
					.ok_or(anyhow::anyhow!("parent directory of file path does not exist"))?,
			)
			.await?;

			tokio::fs::write(&path, format!("hello world {}", i)).await?;
			paths.push(path);
		}

		let manifest = PackageElement { sync_files: paths.clone(), root_dir: root_dir.clone() };

		let package = Package(vec![manifest.clone(), manifest.clone()]);

		// upload the manifest
		let _uploaded_package = push.push(package.clone()).await?;

		// delete the files locally by emptying the tempdir
		tokio::fs::remove_dir_all(&root_dir).await?;

		// download the manifest
		let _downloaded_package = pull.pull(package).await?; // package doesn't really matter here

		// check that all the files are back
		for (i, path) in paths.iter().enumerate() {
			// exists
			assert!(path.exists());

			// content is the same
			let content = tokio::fs::read_to_string(&path).await?;
			assert_eq!(content, format!("hello world {}", i));
		}

		// destroy the backend unforced and catch the error
		let result = push.bucket_connection.destroy(false).await;
		assert!(result.is_err());

		// destroy the backend forced
		pull.bucket_connection.destroy(true).await?;

		Ok(())
	}
}
