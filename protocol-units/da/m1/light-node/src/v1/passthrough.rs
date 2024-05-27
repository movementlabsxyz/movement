use crate::v1::LightNodeV1Operations;
use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::{blob::GasPrice, nmt::Namespace, Blob as CelestiaBlob};
use m1_da_light_node_grpc::*;
use m1_da_light_node_grpc::{blob_response, light_node_service_server::LightNodeService};
use m1_da_light_node_util::Config;
use m1_da_light_node_verifier::{v1::V1Verifier, Verifier};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::{Stream, StreamExt};

#[derive(Clone)]
pub struct LightNodeV1 {
	pub celestia_url: String,
	pub celestia_token: String,
	pub celestia_namespace: Namespace,
	pub default_client: Arc<Client>,
	pub verification_mode: Arc<RwLock<VerificationMode>>,
	pub verifier: Arc<Box<dyn Verifier + Send + Sync>>,
}

#[async_trait::async_trait]
impl LightNodeV1Operations for LightNodeV1 {
	/// Tries to create a new LightNodeV1 instance from the environment variables.
	async fn try_from_env() -> Result<Self, anyhow::Error> {
		let config = Config::try_from_env()?;
		let client = Arc::new(config.connect_celestia().await?);

		Ok(Self {
			celestia_url: config.celestia_url,
			celestia_token: config.celestia_token,
			celestia_namespace: config.celestia_namespace,
			default_client: client.clone(),
			verification_mode: Arc::new(RwLock::new(config.verification_mode)),
			verifier: Arc::new(Box::new(V1Verifier {
				client,
				namespace: config.celestia_namespace.clone(),
			})),
		})
	}

	/// Runs background tasks for the LightNodeV1 instance.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}

impl LightNodeV1 {
	/// Gets a new Celestia client instance with the matching params.
	pub async fn get_new_celestia_client(&self) -> Result<Client, anyhow::Error> {
		Client::new(&self.celestia_url, Some(&self.celestia_token))
			.await
			.map_err(|e| anyhow::anyhow!("Failed to create Celestia client: {}", e))
	}

	/// Creates a new blob instance with the provided data.
	pub fn create_new_celestia_blob(&self, data: Vec<u8>) -> Result<CelestiaBlob, anyhow::Error> {
		CelestiaBlob::new(self.celestia_namespace, data)
			.map_err(|e| anyhow::anyhow!("Failed to create a blob: {}", e))
	}

	/// Submits a CelestiaNlob to the Celestia node.
	pub async fn submit_celestia_blob(&self, blob: CelestiaBlob) -> Result<u64, anyhow::Error> {
		let height = self
			.default_client
			.blob_submit(&[blob], GasPrice::default())
			.await
			.map_err(|e| anyhow::anyhow!("Failed submitting the blob: {}", e))?;

		Ok(height)
	}

	/// Submits a blob to the Celestia node.
	pub async fn submit_blob(&self, data: Vec<u8>) -> Result<Blob, anyhow::Error> {
		let celestia_blob = self.create_new_celestia_blob(data)?;
		let height = self.submit_celestia_blob(celestia_blob.clone()).await?;
		Ok(Self::celestia_blob_to_blob(celestia_blob, height)?)
	}

	/// Gets the blobs at a given height.
	pub async fn get_celestia_blobs_at_height(
		&self,
		height: u64,
	) -> Result<Vec<CelestiaBlob>, anyhow::Error> {
		let blobs = self.default_client.blob_get_all(height, &[self.celestia_namespace]).await;

		if blobs.is_err() {
			println!("Error getting blobs: {:?}", blobs.as_ref().err().unwrap());
		}

		let blobs = blobs.unwrap_or_default();

		let mut verified_blobs = Vec::new();
		for blob in blobs {
			println!("Verifying blob");

			let blob_data = blob.data.clone();

			// todo: improve error boundary here to detect crashes
			let verified = self
				.verifier
				.verify(*self.verification_mode.read().await, &blob_data, height)
				.await;

			if verified.is_err() {
				println!("Error verifying blob: {:?}", verified.as_ref().err().unwrap());
			}

			let verified = verified.unwrap_or(true);

			if verified {
				verified_blobs.push(blob);
			}
		}

		Ok(verified_blobs)
	}

	pub async fn get_blobs_at_height(&self, height: u64) -> Result<Vec<Blob>, anyhow::Error> {
		let celestia_blobs = self.get_celestia_blobs_at_height(height).await?;
		let mut blobs = Vec::new();
		for blob in celestia_blobs {
			blobs.push(Self::celestia_blob_to_blob(blob, height)?);
		}
		Ok(blobs)
	}

	/// Streams blobs until it can't get another one in the loop
	pub async fn stream_blobs_in_range(
		&self,
		start_height: u64,
		end_height: Option<u64>,
	) -> Result<
		std::pin::Pin<Box<dyn Stream<Item = Result<Blob, anyhow::Error>> + Send>>,
		anyhow::Error,
	> {
		let mut height = start_height;
		let end_height = end_height.unwrap_or_else(|| u64::MAX);
		let me = Arc::new(self.clone());

		let stream = async_stream::try_stream! {
			loop {
				if height > end_height {
					break;
				}

				let blobs = me.get_blobs_at_height(height).await?;
				for blob in blobs {
					yield blob;
				}
				height += 1;
			}
		};

		Ok(Box::pin(stream)
			as std::pin::Pin<Box<dyn Stream<Item = Result<Blob, anyhow::Error>> + Send>>)
	}

	/// Streams the latest blobs that can subscribed to.
	async fn stream_blobs_from_height_on(
		&self,
		start_height: Option<u64>,
	) -> Result<
		std::pin::Pin<Box<dyn Stream<Item = Result<Blob, anyhow::Error>> + Send>>,
		anyhow::Error,
	> {
		let start_height = start_height.unwrap_or_else(|| u64::MAX);
		let me = Arc::new(self.clone());
		let mut subscription = me.default_client.header_subscribe().await?;

		let stream = async_stream::try_stream! {
			let mut first_flag = true;
			while let Some(header_res) = subscription.next().await {

				let header = header_res?;
				let height = header.height().into();
				println!("Stream got header: {:?}", header.height());

				// back fetch the blobs
				if first_flag && (height > start_height) {

					let mut blob_stream = me.stream_blobs_in_range(start_height, Some(height)).await?;

					while let Some(blob) = blob_stream.next().await {
						println!("Stream got blob: {:?}", blob);
						yield blob?;
					}

				}
				first_flag = false;

				let blobs = me.get_blobs_at_height(height).await?;
				for blob in blobs {
					println!("Stream got blob: {:?}", blob);
					yield blob;
				}
			}
		};

		Ok(Box::pin(stream)
			as std::pin::Pin<Box<dyn Stream<Item = Result<Blob, anyhow::Error>> + Send>>)
	}

	pub fn celestia_blob_to_blob(blob: CelestiaBlob, height: u64) -> Result<Blob, anyhow::Error> {
		Ok(Blob {
			data: blob.data,
			blob_id: serde_json::to_string(&blob.commitment)
				.map_err(|e| anyhow::anyhow!("Failed to serialize commitment: {}", e))?,
			height,
		})
	}

	pub fn blob_to_blob_write_response(blob: Blob) -> Result<BlobResponse, anyhow::Error> {
		Ok(BlobResponse { blob_type: Some(blob_response::BlobType::PassedThroughBlob(blob)) })
	}

	pub fn blob_to_blob_read_response(blob: Blob) -> Result<BlobResponse, anyhow::Error> {
		#[cfg(feature = "sequencer")]
		{
			Ok(BlobResponse { blob_type: Some(blob_response::BlobType::SequencedBlobBlock(blob)) })
		}

		#[cfg(not(feature = "sequencer"))]
		{
			Ok(BlobResponse { blob_type: Some(blob_response::BlobType::PassedThroughBlob(blob)) })
		}
	}
}

#[tonic::async_trait]
impl LightNodeService for LightNodeV1 {
	/// Server streaming response type for the StreamReadFromHeight method.
	type StreamReadFromHeightStream = std::pin::Pin<
		Box<
			dyn Stream<Item = Result<StreamReadFromHeightResponse, tonic::Status>> + Send + 'static,
		>,
	>;

	/// Stream blobs from a specified height or from the latest height.
	async fn stream_read_from_height(
		&self,
		request: tonic::Request<StreamReadFromHeightRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadFromHeightStream>, tonic::Status> {
		let me = Arc::new(self.clone());
		let height = request.into_inner().height;

		let output = async_stream::try_stream! {

			let mut blob_stream = me.stream_blobs_from_height_on(Some(height)).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

			while let Some(blob) = blob_stream.next().await {
				let blob = blob.map_err(|e| tonic::Status::internal(e.to_string()))?;
				let response = StreamReadFromHeightResponse {
					blob : Some(Self::blob_to_blob_read_response(blob).map_err(|e| tonic::Status::internal(e.to_string()))?)
				};
				yield response;
			}

		};

		Ok(tonic::Response::new(Box::pin(output) as Self::StreamReadFromHeightStream))
	}

	/// Server streaming response type for the StreamReadLatest method.
	type StreamReadLatestStream = std::pin::Pin<
		Box<dyn Stream<Item = Result<StreamReadLatestResponse, tonic::Status>> + Send + 'static>,
	>;

	/// Stream the latest blobs.
	async fn stream_read_latest(
		&self,
		_request: tonic::Request<StreamReadLatestRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadLatestStream>, tonic::Status> {
		let me = Arc::new(self.clone());

		let output = async_stream::try_stream! {

			let mut blob_stream = me.stream_blobs_from_height_on(None).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
			while let Some(blob) = blob_stream.next().await {
				let blob = blob.map_err(|e| tonic::Status::internal(e.to_string()))?;
				let response = StreamReadLatestResponse {
					blob : Some(Self::blob_to_blob_read_response(blob).map_err(|e| tonic::Status::internal(e.to_string()))?)
				};
				yield response;
			}

		};

		Ok(tonic::Response::new(Box::pin(output) as Self::StreamReadLatestStream))
	}
	/// Server streaming response type for the StreamWriteCelestiaBlob method.
	type StreamWriteBlobStream = std::pin::Pin<
		Box<dyn Stream<Item = Result<StreamWriteBlobResponse, tonic::Status>> + Send + 'static>,
	>;
	/// Stream blobs out, either individually or in batches.
	async fn stream_write_blob(
		&self,
		request: tonic::Request<tonic::Streaming<StreamWriteBlobRequest>>,
	) -> std::result::Result<tonic::Response<Self::StreamWriteBlobStream>, tonic::Status> {
		let mut stream = request.into_inner();
		let me = Arc::new(self.clone());

		let output = async_stream::try_stream! {

			while let Some(request) = stream.next().await {
				let request = request?;
				let blob_data = request.blob.ok_or(tonic::Status::invalid_argument("No blob in request"))?.data;

				let blob = me.submit_blob(blob_data).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

				let write_response = StreamWriteBlobResponse {
					blob : Some(Self::blob_to_blob_read_response(blob).map_err(|e| tonic::Status::internal(e.to_string()))?)
				};

				yield write_response;

			}
		};

		Ok(tonic::Response::new(Box::pin(output) as Self::StreamWriteBlobStream))
	}
	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		request: tonic::Request<ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		let height = request.into_inner().height;
		let blobs = self
			.get_blobs_at_height(height)
			.await
			.map_err(|e| tonic::Status::internal(e.to_string()))?;

		if blobs.is_empty() {
			return Err(tonic::Status::not_found("No blobs found at the specified height"));
		}

		let mut blob_responses = Vec::new();
		for blob in blobs {
			blob_responses.push(
				Self::blob_to_blob_read_response(blob)
					.map_err(|e| tonic::Status::internal(e.to_string()))?,
			);
		}

		Ok(tonic::Response::new(ReadAtHeightResponse {
			// map blobs to the response type
			blobs: blob_responses,
		}))
	}
	/// Batch read and write operations for efficiency.
	async fn batch_read(
		&self,
		request: tonic::Request<BatchReadRequest>,
	) -> std::result::Result<tonic::Response<BatchReadResponse>, tonic::Status> {
		let heights = request.into_inner().heights;
		let mut responses = Vec::with_capacity(heights.len());
		for height in heights {
			let blobs = self
				.get_blobs_at_height(height)
				.await
				.map_err(|e| tonic::Status::internal(e.to_string()))?;

			if blobs.is_empty() {
				return Err(tonic::Status::not_found("No blobs found at the specified height"));
			}

			let mut blob_responses = Vec::new();
			for blob in blobs {
				blob_responses.push(
					Self::blob_to_blob_read_response(blob)
						.map_err(|e| tonic::Status::internal(e.to_string()))?,
				);
			}

			responses.push(ReadAtHeightResponse { blobs: blob_responses })
		}

		Ok(tonic::Response::new(BatchReadResponse { responses }))
	}

	/// Batch write blobs.
	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		let blobs = request.into_inner().blobs;
		let mut responses = Vec::with_capacity(blobs.len());
		for data in blobs {
			let blob = self
				.submit_blob(data.data)
				.await
				.map_err(|e| tonic::Status::internal(e.to_string()))?;
			responses.push(blob);
		}

		let mut blob_responses = Vec::new();
		for blob in responses {
			blob_responses.push(
				Self::blob_to_blob_write_response(blob)
					.map_err(|e| tonic::Status::internal(e.to_string()))?,
			);
		}

		Ok(tonic::Response::new(BatchWriteResponse { blobs: blob_responses }))
	}
	/// Update and manage verification parameters.
	async fn update_verification_parameters(
		&self,
		request: tonic::Request<UpdateVerificationParametersRequest>,
	) -> std::result::Result<tonic::Response<UpdateVerificationParametersResponse>, tonic::Status> {
		let verification_mode = request.into_inner().mode();
		let mut mode = self.verification_mode.write().await;
		*mode = verification_mode;

		Ok(tonic::Response::new(UpdateVerificationParametersResponse {
			mode: verification_mode.into(),
		}))
	}
}

