use aws_sdk_s3::types::{BucketLocationConstraint, CreateBucketConfiguration};

#[derive(Debug, Clone)]
pub struct BucketConnection {
	pub client: aws_sdk_s3::Client,
	pub bucket: String,
}

impl BucketConnection {
	pub fn new(client: aws_sdk_s3::Client, bucket: String) -> Self {
		Self { client, bucket }
	}

	pub(crate) async fn create_bucket_if_not_exists(&self) -> Result<(), anyhow::Error> {
		let bucket = self.bucket.clone();
		let bucket_exists = self.client.head_bucket().bucket(bucket.clone()).send().await.is_ok();
		if !bucket_exists {
			let bucket_builder = CreateBucketConfiguration::builder();

			let bucket_configuration = match self.client.config().region() {
				Some(region) => {
					let constraint = BucketLocationConstraint::from(region.as_ref());
					bucket_builder.location_constraint(constraint).build()
				}
				None => bucket_builder.build(),
			};

			self.client
				.create_bucket()
				.create_bucket_configuration(bucket_configuration)
				.bucket(bucket)
				.send()
				.await?;
		}
		Ok(())
	}

	pub(crate) async fn empty_bucket_if_exists(&self) -> Result<(), anyhow::Error> {
		let bucket = self.bucket.clone();
		let bucket_exists = self.client.head_bucket().bucket(bucket.clone()).send().await.is_ok();
		if bucket_exists {
			let mut continuation_token = None;
			loop {
				let list_objects_output = self
					.client
					.list_objects_v2()
					.bucket(bucket.clone())
					.set_continuation_token(continuation_token)
					.send()
					.await?;
				if let Some(contents) = list_objects_output.contents {
					for object in contents {
						if let Some(key) = object.key {
							self.client
								.delete_object()
								.bucket(bucket.clone())
								.key(key)
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

	pub(crate) async fn destroy_if_exists(&self) -> Result<(), anyhow::Error> {
		let bucket = self.bucket.clone();
		let bucket_exists = self.client.head_bucket().bucket(bucket.clone()).send().await.is_ok();
		if bucket_exists {
			self.client.delete_bucket().bucket(bucket).send().await?;
		}
		Ok(())
	}

	pub async fn create(client: aws_sdk_s3::Client, bucket: String) -> Result<Self, anyhow::Error> {
		let connection = Self::new(client, bucket);
		connection.create_bucket_if_not_exists().await?;
		Ok(connection)
	}

	pub async fn destroy(self, force: bool) -> Result<(), anyhow::Error> {
		if force {
			self.empty_bucket_if_exists().await?;
		}
		self.destroy_if_exists().await?;
		Ok(())
	}
}

#[cfg(test)]
pub mod test {
	use super::BucketConnection;

	#[tokio::test]
	async fn test_create() -> Result<(), anyhow::Error> {
		let bucket = format!("public-test-bucket-{}", uuid::Uuid::new_v4());
		let config = aws_config::load_from_env().await;
		let client = aws_sdk_s3::Client::new(&config);

		let connection = BucketConnection::new(client.clone(), bucket.clone());

		connection.create_bucket_if_not_exists().await?;

		// assert bucket exists
		let bucket_exists = client.head_bucket().bucket(bucket.clone()).send().await.is_ok();
		assert!(bucket_exists);

		connection.create_bucket_if_not_exists().await?;

		// assert bucket still exists
		let bucket_exists = client.head_bucket().bucket(bucket.clone()).send().await.is_ok();
		assert!(bucket_exists);

		connection.destroy_if_exists().await?;

		// assert bucket does not exist
		let bucket_exists = client.head_bucket().bucket(bucket.clone()).send().await.is_ok();
		assert!(!bucket_exists);

		Ok(())
	}
}
