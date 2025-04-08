use crate::batch::FullNodeTxs;
use crate::batch::{validate_batch, DaBatch, RawData};
use crate::block::{BlockHeight, SequencerBlock};
use crate::whitelist::Whitelist;
use crate::DaSequencerError;
use movement_da_sequencer_proto::da_sequencer_node_service_server::{
	DaSequencerNodeService, DaSequencerNodeServiceServer,
};
use movement_da_sequencer_proto::{
	block_response::BlockType, BatchWriteRequest, BatchWriteResponse, BlockResponse, Blockv1,
	ReadAtHeightRequest, ReadAtHeightResponse, StreamReadFromHeightRequest,
	StreamReadFromHeightResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::Stream;
use tonic::transport::Server;

/// Runs the server
pub async fn run_server(
	address: SocketAddr,
	request_tx: mpsc::Sender<GrpcRequests>,
	whitelist: Whitelist,
) -> Result<(), anyhow::Error> {
	tracing::info!("Server listening on: {}", address);
	let whitelist = Arc::new(RwLock::new(whitelist));
	let reflection = tonic_reflection::server::Builder::configure()
		.register_encoded_file_descriptor_set(movement_da_sequencer_proto::FILE_DESCRIPTOR_SET)
		.build_v1()?;

	Server::builder()
		.max_frame_size(1024 * 1024 * 16 - 1)
		.accept_http1(true)
		.add_service(DaSequencerNodeServiceServer::new(DaSequencerNode { request_tx, whitelist }))
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
	whitelist: Arc<RwLock<Whitelist>>,
}

#[tonic::async_trait]
impl DaSequencerNodeService for DaSequencerNode {
	/// Server streaming response type for the StreamReadFromHeight method.
	type StreamReadFromHeightStream = std::pin::Pin<
		Box<
			dyn Stream<Item = Result<StreamReadFromHeightResponse, tonic::Status>> + Send + 'static,
		>,
	>;

	/// Stream blocks from a specified height or from the latest height.
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
					BlockResponse { block_type: Some(BlockType::Blockv1(blockv1)) }

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
							BlockResponse { block_type: Some(BlockType::Heartbeat(true)) }
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
							BlockResponse { block_type: Some(BlockType::Blockv1(blockv1)) }
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

	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		let batch_data = request.into_inner().data;

		// Try to deserialize the batch
		let (public_key, signature, bytes) =
			match movement_da_sequencer_client::deserialize_full_node_batch(batch_data).map_or_else(
				|err| {
					tracing::warn!("Invalid batch send: deserialization failed: {err}");
					None
				},
				|res| Some(res),
			) {
				Some(res) => res,
				None => return Ok(tonic::Response::new(BatchWriteResponse { answer: false })),
			};

		// Validate the batch
		let validated = {
			let whitelist = self.whitelist.read().await;
			let raw_batch = DaBatch::<RawData>::now(public_key, signature, bytes);
			match validate_batch(raw_batch, &whitelist).map_or_else(
				|err| {
					tracing::warn!("Invalid batch send: validation failed: {err}");
					None
				},
				|validated| Some(validated),
			) {
				Some(validated) => validated,
				None => return Ok(tonic::Response::new(BatchWriteResponse { answer: false })),
			}
		};

		if let Err(err) = self.request_tx.send(GrpcRequests::WriteBatch(validated)).await {
			tracing::error!(
				"Internal grpc request channel closed, no more batches will be processed: {err}"
			);
			return Ok(tonic::Response::new(BatchWriteResponse { answer: false }));
		}

		Ok(tonic::Response::new(BatchWriteResponse { answer: true }))
	}

	/// Read one block at a specified height.
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
			block_id: block.get_block_digest().into_vec(),
			height: block.height.into(),
			data: block.try_into()?,
		})
	}
}
