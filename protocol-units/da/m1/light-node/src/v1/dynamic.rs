use crate::v1::{passthrough, sequencer, LightNodeV1Operations};
use m1_da_light_node_grpc::light_node_service_server::LightNodeService;
use m1_da_light_node_grpc::*;
use tokio_stream::Stream;

#[derive(Clone)]
pub enum LightNodeV1 {
	PassThrough(passthrough::LightNodeV1),
	Sequencer(sequencer::LightNodeV1),
}

#[async_trait::async_trait]
impl LightNodeV1Operations for LightNodeV1 {
	async fn try_from_env() -> Result<Self, anyhow::Error> {
		let which =
			std::env::var("M1_DA_LIGHT_NODE_MODE").unwrap_or_else(|_| "passthrough".to_string());

		match which.as_str() {
			"passthrough" => Ok(Self::PassThrough(passthrough::LightNodeV1::try_from_env().await?)),
			"sequencer" => Ok(Self::Sequencer(sequencer::LightNodeV1::try_from_env().await?)),
			_ => Err(anyhow::anyhow!("Unknown mode: {}", which)),
		}
	}

	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		match self {
			Self::PassThrough(pass_through) => pass_through.run_background_tasks().await,
			Self::Sequencer(sequencer) => sequencer.run_background_tasks().await,
		}
	}
}

impl LightNodeV1 {
	pub async fn try_from_env() -> Result<Self, anyhow::Error> {
		let which =
			std::env::var("M1_DA_LIGHT_NODE_MODE").unwrap_or_else(|_| "passthrough".to_string());

		match which.as_str() {
			"passthrough" => Ok(Self::PassThrough(passthrough::LightNodeV1::try_from_env().await?)),
			"sequencer" => Ok(Self::Sequencer(sequencer::LightNodeV1::try_from_env().await?)),
			_ => Err(anyhow::anyhow!("Unknown mode: {}", which)),
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
		match self {
			Self::PassThrough(pass_through) => pass_through.stream_read_from_height(request).await,
			Self::Sequencer(sequencer) => sequencer.stream_read_from_height(request).await,
		}
	}

	/// Server streaming response type for the StreamReadLatest method.
	type StreamReadLatestStream = std::pin::Pin<
		Box<dyn Stream<Item = Result<StreamReadLatestResponse, tonic::Status>> + Send + 'static>,
	>;

	/// Stream the latest blobs.
	async fn stream_read_latest(
		&self,
		request: tonic::Request<StreamReadLatestRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadLatestStream>, tonic::Status> {
		match self {
			Self::PassThrough(pass_through) => pass_through.stream_read_latest(request).await,
			Self::Sequencer(sequencer) => sequencer.stream_read_latest(request).await,
		}
	}
	/// Server streaming response type for the StreamWriteCelestiaBlob method.
	type StreamWriteBlobStream = std::pin::Pin<
		Box<dyn Stream<Item = Result<StreamWriteBlobResponse, tonic::Status>> + Send + 'static>,
	>;
	/// Stream blobs out, either individually or in batches.
	async fn stream_write_blob(
		&self,
		_request: tonic::Request<tonic::Streaming<StreamWriteBlobRequest>>,
	) -> std::result::Result<tonic::Response<Self::StreamWriteBlobStream>, tonic::Status> {
		unimplemented!("StreamWriteBlob not implemented")
	}
	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		request: tonic::Request<ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		match self {
			Self::PassThrough(pass_through) => pass_through.read_at_height(request).await,
			Self::Sequencer(sequencer) => sequencer.read_at_height(request).await,
		}
	}
	/// Batch read and write operations for efficiency.
	async fn batch_read(
		&self,
		request: tonic::Request<BatchReadRequest>,
	) -> std::result::Result<tonic::Response<BatchReadResponse>, tonic::Status> {
		match self {
			Self::PassThrough(pass_through) => pass_through.batch_read(request).await,
			Self::Sequencer(sequencer) => sequencer.batch_read(request).await,
		}
	}

	/// Batch write blobs.
	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		match self {
			Self::PassThrough(pass_through) => pass_through.batch_write(request).await,
			Self::Sequencer(sequencer) => sequencer.batch_write(request).await,
		}
	}
	/// Update and manage verification parameters.
	async fn update_verification_parameters(
		&self,
		request: tonic::Request<UpdateVerificationParametersRequest>,
	) -> std::result::Result<tonic::Response<UpdateVerificationParametersResponse>, tonic::Status> {
		match self {
			Self::PassThrough(pass_through) => {
				pass_through.update_verification_parameters(request).await
			},
			Self::Sequencer(sequencer) => sequencer.update_verification_parameters(request).await,
		}
	}
}

