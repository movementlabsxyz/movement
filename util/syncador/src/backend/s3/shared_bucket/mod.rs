use super::bucket_connection;
use aws_types::region::Region;
use tracing::info;

const UPLOAD_COMPLETE_MARKER_FILE_NAME: &str = "upload_complete.txt";
pub(crate) const DEFAULT_CHUNK_SIZE: usize = 500 * 1024 * 1024; // 500 MB per chunk (adjustable)
pub(crate) const BUFFER_SIZE: usize = 10 * 1024 * 1024; // 10 MB buffer for each read/write operation

pub mod metadata;
pub mod pull;
pub mod push;
use movement_types::application;
use std::path::PathBuf;

pub async fn create_with_load_from_env(
	bucket: String,
	pull_destination: PathBuf,
	metadata: metadata::Metadata,
) -> Result<(push::Push, pull::Pull), anyhow::Error> {
	let region = match std::env::var("AWS_REGION") {
		Ok(region) => Some(Region::new(region)),
		Err(_) => None,
	};
	let config = aws_config::load_from_env().await.into_builder().region(region).build();
	info!("Create client used region {:?}", config.region());
	let client = aws_sdk_s3::Client::new(&config);
	create(client, bucket, metadata, pull_destination).await
}

pub async fn create_push_with_load_from_env(
	bucket: String,
	metadata: metadata::Metadata,
) -> Result<push::Push, anyhow::Error> {
	let region = match std::env::var("AWS_REGION") {
		Ok(region) => Some(Region::new(region)),
		Err(_) => None,
	};
	let config = aws_config::load_from_env().await.into_builder().region(region).build();
	info!("Create client used region {:?}", config.region());
	let client = aws_sdk_s3::Client::new(&config);
	let bucket_connection = bucket_connection::BucketConnection::create(client, bucket).await?;
	let push = push::Push::new(bucket_connection, metadata);
	Ok(push)
}

pub async fn create_pull_with_load_from_env(
	bucket: String,
	metadata: metadata::Metadata,
	pull_destination: PathBuf,
) -> Result<pull::Pull, anyhow::Error> {
	let region = match std::env::var("AWS_REGION") {
		Ok(region) => Some(Region::new(region)),
		Err(_) => None,
	};
	let config = aws_config::load_from_env().await.into_builder().region(region).build();
	info!("Create client used region {:?}", config.region());
	let client = aws_sdk_s3::Client::new(&config);
	let bucket_connection = bucket_connection::BucketConnection::create(client, bucket).await?;
	let pull = pull::Pull::new(bucket_connection, metadata, pull_destination);
	Ok(pull)
}

pub async fn destroy_with_load_from_env(bucket: String) -> Result<(), anyhow::Error> {
	let region = match std::env::var("AWS_REGION") {
		Ok(region) => Some(Region::new(region)),
		Err(_) => None,
	};
	let config = aws_config::load_from_env().await.into_builder().region(region).build();
	info!("Destroy client used region {:?}", config.region());
	let client = aws_sdk_s3::Client::new(&config);
	let bucket_connection = bucket_connection::BucketConnection::new(client, bucket);
	bucket_connection.destroy(true).await
}

pub async fn create_random(
	bucket: String,
	pull_destination: PathBuf,
) -> Result<(push::Push, pull::Pull), anyhow::Error> {
	let metadata = metadata::Metadata::random();
	create_with_load_from_env(bucket, pull_destination, metadata).await
}

pub async fn create_random_with_application_id(
	bucket: String,
	application_id: application::Id,
	pull_destination: PathBuf,
) -> Result<(push::Push, pull::Pull), anyhow::Error> {
	let metadata = metadata::Metadata::random().with_application_id(application_id);
	create_with_load_from_env(bucket, pull_destination, metadata).await
}

pub async fn create(
	client: aws_sdk_s3::Client,
	bucket: String,
	metadata: metadata::Metadata,
	pull_destination: PathBuf,
) -> Result<(push::Push, pull::Pull), anyhow::Error> {
	let bucket_connection = bucket_connection::BucketConnection::create(client, bucket).await?;

	let push = push::Push::new(bucket_connection.clone(), metadata.clone());
	let pull = pull::Pull::new(bucket_connection, metadata, pull_destination);

	Ok((push, pull))
}

#[cfg(test)]
pub mod test {
	//! pub in case we want to reuse helpers

	use super::*;
	use crate::backend::s3::bucket_connection::BucketConnection;
	use crate::backend::s3::shared_bucket::pull::Pull;
	use crate::backend::s3::shared_bucket::push::Push;
	use crate::backend::{PullOperations, PushOperations};
	use crate::files::package::{Package, PackageElement};
	use movement_types::actor;
	use std::fs::File;
	use std::io::BufReader;
	use std::io::BufWriter;
	use std::io::Read;
	use std::io::Write;

	#[tokio::test]
	pub async fn test_archive_split() -> Result<(), anyhow::Error> {
		use tracing_subscriber::EnvFilter;

		tracing_subscriber::fmt()
			.with_env_filter(
				EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
			)
			.init();
		// 1) Chunk size is bigger than the archive. No split in chunk.
		process_archive_test("test_archive_split.tmp", 10 * 1024, 600).await?;
		// 2) Chunk size is smaller than the archive. Several chunk is create and reconstructed.
		process_archive_test("test_archive_split2.tmp", 1024, 312).await?;
		Ok(())
	}

	async fn process_archive_test(
		temp_file_name: &str,
		chunk_size: usize,
		buffer_size: usize,
	) -> Result<(), anyhow::Error> {
		//Create source and destination temp dir.
		let source_dir = tempfile::tempdir()?;
		let destination_dir = tempfile::tempdir()?;

		//create file to Push size 10 * 1024.
		let archive_file_path = source_dir.path().join(temp_file_name);
		{
			let data: Vec<u8> = (0..1024usize).map(|i| (i % 256) as u8).collect();
			let file = File::create(&archive_file_path)?;
			let mut writer = BufWriter::new(file);
			//Fill with some data. 10 Mb
			(0..10).try_for_each(|_| writer.write_all(&data))?;
		}

		let bucket = format!("public-test-bucket-{}", uuid::Uuid::new_v4());
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_s3::Client::new(&config);

		let connection = BucketConnection::new(client.clone(), bucket.clone());

		connection.create_bucket_if_not_exists().await?;

		let application_id = application::Id::new([
			26, 43, 60, 77, 94, 111, 122, 139, 156, 173, 190, 207, 208, 225, 242, 3, 20, 37, 54,
			71, 88, 105, 122, 139, 156, 173, 190, 207, 208, 225, 242, 3,
		]);
		let syncer_id = actor::Id::new([
			10, 64, 193, 217, 99, 233, 100, 32, 31, 1, 244, 166, 56, 79, 213, 208, 112, 158, 162,
			27, 10, 111, 130, 99, 91, 130, 103, 26, 12, 121, 210, 75,
		]);

		let metadata = metadata::Metadata::default()
			.with_application_id(application_id)
			.with_syncer_id(syncer_id);

		let push = Push {
			bucket_connection: connection,
			metadata: metadata.clone(),
			chunk_size,
			buffer_size,
		};

		let element = PackageElement {
			sync_files: vec![archive_file_path],
			root_dir: source_dir.path().to_path_buf(),
		};
		let package = Package(vec![element]);
		let archive_package = push.push(package).await?;

		// Pull archive
		let connection = BucketConnection::new(client.clone(), bucket.clone());
		let pull = Pull::new(connection, metadata, destination_dir.path().to_path_buf());
		let element = PackageElement {
			sync_files: vec![archive_package.0[0].sync_files[0].clone()],
			root_dir: destination_dir.path().to_path_buf(),
		};
		let package = Package(vec![element]);

		let dest_package = pull
			.pull(Some(package.clone()))
			.await?
			.ok_or(anyhow::anyhow!("Error No file pulled."))?;

		verify_archive(&dest_package.0[0].sync_files[0])?;

		//verify that all chunk has been removed
		let has_chunk = std::fs::read_dir(&destination_dir)?
			.find(|entry| {
				entry
					.as_ref()
					.ok()
					.map(|entry| {
						let path = entry.path();
						path.is_file()
							&& path.extension().and_then(|ext| ext.to_str()) == Some("chunk")
					})
					.unwrap_or(false)
			})
			.is_some();
		assert!(!has_chunk, "Some chunk are still present.");

		//Do a second pull to validate it manage last pull remaining files.
		let dest_package = pull
			.pull(Some(package))
			.await?
			.ok_or(anyhow::anyhow!("Error second pull failed."))?;
		verify_archive(&dest_package.0[0].sync_files[0])?;

		Ok(())
	}

	fn verify_archive<P: AsRef<std::path::Path> + std::marker::Copy>(
		archive_file: P,
	) -> Result<(), anyhow::Error> {
		let file_metadata = std::fs::metadata(archive_file)?;
		let file_size = file_metadata.len() as usize;
		assert_eq!(file_size, 10 * 1024, "dest file hasn't the right size: {file_size}");

		//verify that the file byte are in order.
		let pulled_file = File::open(archive_file)?;
		let mut reader = BufReader::new(pulled_file);
		let mut buffer = [0u8; 1024];
		let mut expected_byte: u8 = 0;

		loop {
			let bytes_read = reader.read(&mut buffer)?;
			if bytes_read == 0 {
				break; // End of file
			}

			for &byte in &buffer[..bytes_read] {
				if byte != expected_byte {
					panic!("Pull file bytes in wrong order.");
				}
				expected_byte = expected_byte.wrapping_add(1); // Increment and wrap around after 255
			}
		}
		Ok(())
	}

	#[tokio::test]
	async fn test_create() -> Result<(), anyhow::Error> {
		// generate a temp pull destination
		let pull_destination = tempfile::tempdir()?.into_path();
		// get is pathbuf
		let pull_destination = pull_destination.to_path_buf();

		// use uuid to generate a random bucket identifier
		let bucket = format!("public-test-bucket-{}", uuid::Uuid::new_v4());
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_s3::Client::new(&config);
		let (_push, pull) =
			create(client.clone(), bucket.clone(), metadata::Metadata::random(), pull_destination)
				.await?;

		// check that the buckets exist
		let bucket_exists = client.head_bucket().bucket(bucket.clone()).send().await.is_ok();
		assert!(bucket_exists);

		let pull::Pull { bucket_connection, metadata: _, pull_destination: _ } = pull;
		bucket_connection.destroy(false).await?;

		// check that the buckets don't exist
		let bucket_exists = client.head_bucket().bucket(bucket.clone()).send().await.is_ok();

		assert!(!bucket_exists);

		Ok(())
	}

	#[tokio::test]
	async fn test_upload_download_many() -> Result<(), anyhow::Error> {
		// create a tempdir
		let tempdir = tempfile::tempdir()?;
		let root_dir = tempdir.path().to_path_buf();

		// use uuid to generate a random bucket identifier
		let bucket = format!("public-test-bucket-{}", uuid::Uuid::new_v4());
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_s3::Client::new(&config);
		let (push, pull) =
			create(client.clone(), bucket.clone(), metadata::Metadata::random(), root_dir.clone())
				.await?;

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
		let _downloaded_package = pull.pull(Some(package)).await?; // package doesn't really matter here

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
