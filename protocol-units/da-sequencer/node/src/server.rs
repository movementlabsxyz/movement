use crate::batch::FullNodeTxs;
use crate::batch::{validate_batch, DaBatch, RawData};
use crate::block::BlockHeight;
use crate::block::SequencerBlock;
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
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::Stream;
use tonic::transport::Server;

/// Runs the server
pub async fn run_server(
	address: SocketAddr,
	request_tx: mpsc::Sender<GrpcRequests>,
) -> Result<(), anyhow::Error> {
	let reflection = tonic_reflection::server::Builder::configure()
		.register_encoded_file_descriptor_set(movement_da_sequencer_proto::FILE_DESCRIPTOR_SET)
		.build_v1()?;

	tracing::info!("Server listening on: {}", address);
	Server::builder()
		.max_frame_size(1024 * 1024 * 16 - 1)
		.accept_http1(true)
		.add_service(DaSequencerNodeServiceServer::new(DaSequencerNode { request_tx }))
		.add_service(reflection)
		.serve(address)
		.await?;

	Ok(())
}

#[derive(Debug)]
pub enum GrpcRequests {
	StartBlockStream { callback: oneshot::Sender<(BlockHeight, mpsc::Receiver<SequencerBlock>)> },
	GetBlockHeight { block_height: BlockHeight, callback: oneshot::Sender<SequencerBlock> },
	WriteBatch(DaBatch<FullNodeTxs>),
}

pub struct DaSequencerNode {
	request_tx: mpsc::Sender<GrpcRequests>,
}

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
		let batch_data = request.into_inner().data;
		let batch = match crate::batch::deserialize_full_node_batch(batch_data).and_then(
			|(public_key, signature, bytes)| {
				validate_batch(DaBatch::<RawData>::now(public_key, signature, bytes))
			},
		) {
			Ok(batch) => batch,
			Err(err) => {
				tracing::warn!("Invalid batch send, verification / validation failed:{err}");
				return Ok(tonic::Response::new(BatchWriteResponse { answer: false }));
			}
		};
		if let Err(err) = self.request_tx.send(GrpcRequests::WriteBatch(batch)).await {
			tracing::error!(
				"Internal grpc request channel closed, no more batch will be processed:{err}"
			);
			return Ok(tonic::Response::new(BatchWriteResponse { answer: false }));
		}
		Ok(tonic::Response::new(BatchWriteResponse { answer: true }))
	}

	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		_request: tonic::Request<ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		Err(tonic::Status::unimplemented(""))
	}
}

pub struct GrpcBatchData {}
