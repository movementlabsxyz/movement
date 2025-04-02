use crate::batch::FullNodeTxs;
use crate::batch::{validate_batch, DaBatch, RawData};
use crate::block::{BlockHeight, SequencerBlock};
use crate::DaSequencerError;
use movement_da_sequencer_proto::da_sequencer_node_service_server::{
	DaSequencerNodeService, DaSequencerNodeServiceServer,
};
use movement_da_sequencer_proto::Blockv1;
use movement_da_sequencer_proto::{blob_response::BlobType, BlobResponse};

use movement_da_sequencer_proto::{
	BatchWriteRequest, BatchWriteResponse, ReadAtHeightRequest, ReadAtHeightResponse,
	StreamReadFromHeightRequest, StreamReadFromHeightResponse,
};
use std::net::SocketAddr;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::Stream;
use tonic::transport::Server;

/// Runs the server
pub async fn run_server(
	address: SocketAddr,
	request_tx: mpsc::Sender<GrpcRequests>,
) -> Result<(), anyhow::Error> {
	tracing::info!("Server listening on: {}", address);
	let reflection = tonic_reflection::server::Builder::configure()
		.register_encoded_file_descriptor_set(movement_da_sequencer_proto::FILE_DESCRIPTOR_SET)
		.build_v1()?;

	tracing::info!("Server started on: {}", address);
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
	StartBlockStream(mpsc::UnboundedSender<Option<SequencerBlock>>, oneshot::Sender<BlockHeight>),
	GetBlockHeight(BlockHeight, oneshot::Sender<Option<SequencerBlock>>),
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
		tracing::info!(request = ?request, "Stream read from height request");

		// Register the new produced block channel to get lastest block.
		// Use `unbounded_channel` to avoid filling the channel during the fetching of an old block.
		let (produced_tx, mut produced_rx) = mpsc::unbounded_channel();
		let (current_height_tx, current_height_rx) = oneshot::channel();
		if let Err(err) = self
			.request_tx
			.send(GrpcRequests::StartBlockStream(produced_tx, current_height_tx))
			.await
		{
			tracing::warn!(error = %err, "Internal grpc request channel closed, can't stream blocks");
			return Err(tonic::Status::internal("Internal error. Retry later"));
		}

		let mut current_produced_height = match current_height_rx.await {
			Ok(h) => h,
			Err(err) => {
				tracing::warn!("start stream channel closed: {err}");
				return Err(tonic::Status::internal("Internal error. Retry later"));
			}
		};
		let mut current_block_height = request.into_inner().height;
		//The genesis block can't be retrieved.
		if current_block_height == 0 {
			current_block_height = 1;
		}
		let request_tx = self.request_tx.clone();
		let output = async_stream::try_stream! {
			loop {
				let response_content = if current_block_height <= current_produced_height.0 {
					//get all block until the current produced height
					let (get_height_tx, get_height_rx) = oneshot::channel();
					if let Err(err) = request_tx.send(GrpcRequests::GetBlockHeight(
						current_block_height.into(),
						get_height_tx,
					)).await {
						tracing::warn!(error = %err, "Request channel closed while requesting GetBlockHeight.");
						return;
					}
					let block = match get_height_rx.await {
						Ok(b) => b,
						Err(err) => {
							tracing::warn!(error = %err, "Stream block: oneshot channel closed");
							return;
						}
					};
					current_block_height +=1;

					let blockv1 = match block.as_ref().map(|block| block.try_into()) {
						None => continue,
						Some(Ok(block)) => block,
						Some(Err(err)) => {
							tracing::warn!(error = %err, "Streamed block serialization failed.");
							return;

						}
					};
					BlobResponse { blob_type: Some(BlobType::Blockv1(blockv1)) }

				} else {
					//send block in produced channel
					let received_content = match produced_rx.recv().await {
						Some(block) => block,
						None => {
							tracing::warn!("Stream block: produced block channel closed.");
							return;
						}

					};

					match received_content {
						None => {
							// send heartbeat.
							BlobResponse { blob_type: Some(BlobType::Heartbeat(true)) }
						}

						Some(new_block) => {
							// send newly produced block.
							if current_block_height + 1 < new_block.height.0 {
								// we missed a block request.
								current_produced_height = new_block.height;
								continue;
							}
							current_block_height = new_block.height.0;

							let blockv1 = match new_block.try_into() {
								Ok(b) => b,
								Err(err) => {
									tracing::warn!(error = %err, "Stream block: block serialization failed.");
									return;

								}
							};
							BlobResponse { blob_type: Some(BlobType::Blockv1(blockv1)) }
						}
					}



				};
				let response = StreamReadFromHeightResponse {
					response: Some(response_content)
				};

				yield response;

			}
		};

		Ok(tonic::Response::new(Box::pin(output) as Self::StreamReadFromHeightStream))
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
				tracing::warn!(error = %err, "Invalid batch send, verification / validation failed.");
				return Ok(tonic::Response::new(BatchWriteResponse { answer: false }));
			}
		};
		if let Err(err) = self.request_tx.send(GrpcRequests::WriteBatch(batch)).await {
			tracing::error!(error = %err, "Internal grpc request channel closed, no more batch will be processed.");
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

impl TryFrom<SequencerBlock> for Blockv1 {
	type Error = DaSequencerError;

	fn try_from(block: SequencerBlock) -> Result<Self, Self::Error> {
		Blockv1::try_from(&block)
	}
}

impl TryFrom<&SequencerBlock> for Blockv1 {
	type Error = DaSequencerError;

	fn try_from(block: &SequencerBlock) -> Result<Self, Self::Error> {
		Ok(Blockv1 {
			blobckid: block.get_block_digest().into_vec(),
			height: block.height.into(),
			data: block.try_into()?,
		})
	}
}
