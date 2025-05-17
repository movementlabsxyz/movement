use crate::batch::FullNodeTxs;
use crate::batch::{validate_batch, DaBatch, RawData};
use crate::block::NodeState;
use crate::block::{BlockHeight, SequencerBlock};
use crate::whitelist::Whitelist;
use crate::DaSequencerError;
use ed25519_dalek::{Verifier, VerifyingKey};
use movement_da_sequencer_client::serialize_node_state;
use movement_da_sequencer_proto::da_sequencer_node_service_server::{
	DaSequencerNodeService, DaSequencerNodeServiceServer,
};
use movement_da_sequencer_proto::{
	block_response::BlockType, BatchWriteRequest, BatchWriteResponse, BlockResponse, BlockV1,
	ReadAtHeightRequest, ReadAtHeightResponse, StreamReadFromHeightRequest,
	StreamReadFromHeightResponse,
};
use movement_da_sequencer_proto::{MainNodeState, MainNodeStateRequest};
use std::net::SocketAddr;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::Stream;
use tonic::transport::Server;

/// Runs the server
pub async fn run_server(
	address: SocketAddr,
	request_tx: mpsc::Sender<GrpcRequests>,
	whitelist: Whitelist,
	main_node_verifying_key: Option<VerifyingKey>,
) -> Result<(), anyhow::Error> {
	tracing::info!("Server listening on: {}", address);
	// Enables gRPC introspection, a feature that allows gRPC clients
	// (like grpcurl, Postman, or IDEs) to query the server for:
	// - Available services and RPC methods
	// - Input/output message types
	// - Field types, enums, and nested structures
	// - Metadata like documentation or options (if included)
	let reflection = tonic_reflection::server::Builder::configure()
		.register_encoded_file_descriptor_set(movement_da_sequencer_proto::FILE_DESCRIPTOR_SET)
		.build_v1()?;

	Server::builder()
		.max_frame_size(1024 * 1024 * 16 - 1)
		.accept_http1(true)
		.add_service(DaSequencerNodeServiceServer::new(DaSequencerNode {
			request_tx,
			whitelist,
			main_node_verifying_key,
		}))
		.add_service(reflection)
		.serve(address)
		.await?;

	Ok(())
}

#[derive(Debug)]
pub enum GrpcRequests {
	StartBlockStream(
		mpsc::UnboundedSender<Option<(SequencerBlock, Option<NodeState>)>>,
		oneshot::Sender<BlockHeight>,
	),
	GetBlockHeight(BlockHeight, oneshot::Sender<Option<SequencerBlock>>),
	WriteBatch(DaBatch<FullNodeTxs>),
	SendState(NodeState),
}

pub struct DaSequencerNode {
	request_tx: mpsc::Sender<GrpcRequests>,
	whitelist: Whitelist,
	main_node_verifying_key: Option<VerifyingKey>,
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
	) -> Result<tonic::Response<Self::StreamReadFromHeightStream>, tonic::Status> {
		tracing::info!(request = ?request, "Stream read from height request");

		// Register the new produced block channel to get lastest block.
		// Use `unbounded_channel` to avoid filling the channel during the fetching of an old block.
		let (current_produced_block_tx, mut current_produced_block_rx) = mpsc::unbounded_channel();
		let (first_produced_block_height_tx, first_produced_block_height_rx) = oneshot::channel();
		if let Err(err) = self
			.request_tx
			.send(GrpcRequests::StartBlockStream(
				current_produced_block_tx,
				first_produced_block_height_tx,
			))
			.await
		{
			tracing::warn!(error = %err, "Internal grpc request channel closed, can't stream blocks");
			return Err(tonic::Status::internal("Internal error. Retry later"));
		}

		// Wait from the supervisor main loop the height of the first produced block that will be sent in the `current_produced_block_rx` channel.
		// Until this block height, block are fetch from the DB and after from the channel.
		// This way blocks are fetch in order from start height until the last produced block.
		// New procured block will arrive from the channel.
		let mut first_produced_block_height = match first_produced_block_height_rx.await {
			Ok(h) => h,
			Err(err) => {
				tracing::warn!("start stream channel closed: {err}");
				return Err(tonic::Status::internal("Internal error. Retry later"));
			}
		};
		let mut current_block_height = request.into_inner().height;

		//The genesis block can't be retrieved so set min height to 1.
		//In the DB block height start as 1 and the genesis block is not present.
		if current_block_height == 0 {
			current_block_height = 1;
		}

		let request_tx = self.request_tx.clone();
		let output = async_stream::try_stream! {
			loop {
				// Needed block height is before the first produced block in the produced block channel.
				let response_content = if current_block_height <= first_produced_block_height.0 {
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

					let block_v1 = match block.map(|block| block.try_into()) {
						None => {
							tracing::error!("Streamed block, get block: {} from DB is missing. Close the stream.",current_block_height);
							return;
						}
						Some(Ok(block)) => block,
						Some(Err(err)) => {
							tracing::warn!(error = %err, "Streamed block serialization failed.");
							return;

						}
					};
					current_block_height +=1;
					BlockResponse { block_type: Some(BlockType::BlockV1(block_v1)) }

				// Fetch new produced block.
				} else {
					//send block in produced channel
					let received_content = match current_produced_block_rx.recv().await {
						Some(block) => block,
						None => {
							tracing::warn!("Streamed block: produced block channel closed.");
							return;
						}

					};

					match received_content {
						None => {
							// send heartbeat.
							BlockResponse { block_type: Some(BlockType::Heartbeat(true)) }
						}

						Some((new_block, state)) => {
							// If the new produced height is not the next one it means that someway we miss blocks.
							// Use the DB fetching mechanism to request them.
							// Set the first_produced_block_height the to missing block height.
							if current_block_height + 1 < new_block.height().0 {
								// we missed a block request it.
								first_produced_block_height = new_block.height();
								tracing::warn!("Streamed block: Produced block fetching miss some blocks");
								tracing::warn!("current_block_height:{current_block_height} produced block height:{}.", new_block.height().0);
								tracing::warn!("Fetch them from the DB.");
								continue;
							}
							current_block_height = new_block.height().0;

							let mut block_v1: BlockV1 = match new_block.try_into() {
								Ok(b) => b,
								Err(err) => {
									tracing::warn!(error = %err, "Stream block: block serialization failed.");
									return;

								}
							};
							// send newly produced block.
							block_v1.node_state = state.map(|s| s.into());
							BlockResponse { block_type: Some(BlockType::BlockV1(block_v1)) }
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

	/// Read one block at a specified height.
	async fn read_at_height(
		&self,
		_request: tonic::Request<ReadAtHeightRequest>,
	) -> Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		Err(tonic::Status::unimplemented(""))
	}

	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
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
			let raw_batch = DaBatch::<RawData>::now(public_key, signature, bytes);
			match validate_batch(raw_batch, &self.whitelist).map_or_else(
				|err| {
					tracing::warn!(
						"Invalid batch send from sender:0x{}.  validation failed: {err}",
						hex::encode(&public_key.to_bytes())
					);
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

	async fn send_state(
		&self,
		request: tonic::Request<MainNodeStateRequest>,
	) -> Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		if self.main_node_verifying_key.is_none() {
			tracing::warn!("Receive a node state and no verifying key is defined.");
			return Ok(tonic::Response::new(BatchWriteResponse { answer: false }));
		}
		let state_data = request.into_inner();

		let state = match state_data.state {
			Some(state) => state,
			None => {
				tracing::warn!("Bad node state data, no state in it.");
				return Ok(tonic::Response::new(BatchWriteResponse { answer: false }));
			}
		};

		let data = serialize_node_state(&state);
		let signature = ed25519_dalek::Signature::try_from(state_data.signature.as_slice())
			.map_err(|err| {
				tonic::Status::new(
					tonic::Code::Unauthenticated,
					format!("send_state bad signature: {err}"),
				)
			})?;
		//unwrap tested just before
		if let Err(err) = self.main_node_verifying_key.as_ref().unwrap().verify(&data, &signature) {
			tracing::warn!("Grpc send_state called with a wrong signature : {err}");
			return Ok(tonic::Response::new(BatchWriteResponse { answer: false }));
		}

		let state =
			NodeState::new(state.block_height, state.ledger_timestamp, state.ledger_version);
		if let Err(err) = self.request_tx.send(GrpcRequests::SendState(state)).await {
			tracing::error!(
				"Internal grpc request channel closed, no more state will be processed: {err}"
			);
			return Ok(tonic::Response::new(BatchWriteResponse { answer: false }));
		}
		Ok(tonic::Response::new(BatchWriteResponse { answer: true }))
	}
}

impl TryFrom<SequencerBlock> for BlockV1 {
	type Error = DaSequencerError;

	fn try_from(block: SequencerBlock) -> Result<Self, Self::Error> {
		Ok(BlockV1 {
			block_id: block.id().to_vec(),
			height: block.height().into(),
			data: bcs::to_bytes(&block.inner_block())
				.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?,
			node_state: None,
		})
	}
}

impl From<NodeState> for MainNodeState {
	fn from(state: NodeState) -> Self {
		MainNodeState {
			block_height: state.block_height,
			ledger_timestamp: state.ledger_timestamp,
			ledger_version: state.ledger_version,
		}
	}
}
