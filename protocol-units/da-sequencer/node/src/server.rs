use movement_da_sequencer_proto::da_sequencer_node_service_server::{
	DaSequencerNodeService, DaSequencerNodeServiceServer,
};
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_da_sequencer_proto::BatchWriteResponse;
use movement_da_sequencer_proto::ReadAtHeightRequest;
use movement_da_sequencer_proto::ReadAtHeightResponse;
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use movement_da_sequencer_proto::StreamReadFromHeightResponse;
use std::net::SocketAddr;
use tokio_stream::Stream;
use tonic::transport::Server;

/// Runs the server
pub async fn run_server(address: SocketAddr) -> Result<(), anyhow::Error> {
	let reflection = tonic_reflection::server::Builder::configure()
		.register_encoded_file_descriptor_set(movement_da_sequencer_proto::FILE_DESCRIPTOR_SET)
		.build_v1()?;

	tracing::info!("Server listening on: {}", address);
	Server::builder()
		.max_frame_size(1024 * 1024 * 16 - 1)
		.accept_http1(true)
		.add_service(DaSequencerNodeServiceServer::new(DaSequencerNode {}))
		.add_service(reflection)
		.serve(address)
		.await?;

	Ok(())
}

pub struct DaSequencerNode {}

#[tonic::async_trait]
impl DaSequencerNodeService for DaSequencerNode {
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
		tracing::info!("Stream read from height request: {:?}", request);
		todo!();
	}

	/// Batch write blobs.
	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		todo!();
	}

	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		_request: tonic::Request<ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		Err(tonic::Status::unimplemented(""))
	}
}
