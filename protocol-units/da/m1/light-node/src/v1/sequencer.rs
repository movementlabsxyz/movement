use tokio_stream::Stream;
use std::{sync::{atomic::{Ordering, AtomicU64}, Arc}, time::Duration};
use tracing::{info, debug};

use std::{fmt::Debug, path::PathBuf};
use celestia_rpc::HeaderClient;
use m1_da_light_node_grpc::light_node_service_server::LightNodeService;
use m1_da_light_node_util::config::Config;
// FIXME: glob imports are bad style
use m1_da_light_node_grpc::*;
use memseq::{Sequencer, Transaction};
use tokio::{time::timeout, sync::mpsc::{Receiver, Sender}};
use movement_types::{Block, algs::grouping_heuristic::{
	GroupingOutcome,
	GroupingHeuristicStack,
	apply::ToApply,
	drop_success::DropSuccess,
	splitting::Splitting,
	binpacking::FirstFitBinpacking,
	skip::SkipFor
}};
use std::boxed::Box;

use crate::v1::{passthrough::LightNodeV1 as LightNodeV1PassThrough, LightNodeV1Operations};

const LOGGING_UID : AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct LightNodeV1 {
	pub pass_through: LightNodeV1PassThrough,
	pub memseq: Arc<memseq::Memseq<memseq::RocksdbMempool>>,
}

impl Debug for LightNodeV1 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("LightNodeV1").field("pass_through", &self.pass_through).finish()
	}
}

impl LightNodeV1Operations for LightNodeV1 {
	async fn try_from_config(config: Config) -> Result<Self, anyhow::Error> {
		info!("Initializing LightNodeV1 in sequencer mode from environment.");

		let pass_through = LightNodeV1PassThrough::try_from_config(config.clone()).await?;
		info!("Initialized pass through for LightNodeV1 in sequencer mode.");

		let memseq_path = pass_through.config.try_memseq_path()?;
		info!("Memseq path: {:?}", memseq_path);
		let (max_block_size, build_time) = pass_through.config.try_block_building_parameters()?;

		let memseq = Arc::new(memseq::Memseq::try_move_rocks(
			PathBuf::from(memseq_path),
			max_block_size,
			build_time,
		)?);
		info!("Initialized Memseq with Move Rocks for LightNodeV1 in sequencer mode.");

		Ok(Self { pass_through, memseq })
	}

	fn try_service_address(&self) -> Result<String, anyhow::Error> {
		self.pass_through.try_service_address()
	}

	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		self.run_block_proposer().await?;

		Ok(())
	}

}

impl LightNodeV1 {

	async fn tick_build_blocks(&self, sender : Sender<Block>) -> Result<(), anyhow::Error> {

		let memseq = self.memseq.clone();

		// this has an internal timeout based on its building time
		// so in the worst case scenario we will roughly double the internal timeout
		let uid = LOGGING_UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		debug!(target: "movement_timing", uid = %uid, "waiting_for_next_block",);
		let block = memseq.wait_for_next_block().await?;
		match block {
			Some(block) => {
				info!(target: "movement_timing", block_id = %block.id(), uid = %uid, transaction_count = block.transactions.len(), "received_block");
				sender.send(block).await?;
				Ok(())
			}
			None => {
				// no transactions to include
				debug!(target: "movement_timing", uid = %uid, "no_transactions_to_include");
				Ok(())
			}
			
		}

	}

	async fn submit_blocks(&self, blocks : &Vec<Block>) -> Result<(), anyhow::Error> {

		let uid = LOGGING_UID.load(Ordering::SeqCst);
		info!(
			target: "movement_timing",
			batch_size = blocks.len(),
			uid = uid, 
			"inner_submitting_block_batch"
		);
		let mut block_blobs = Vec::new();
		for block in blocks {
			let block_blob = self.pass_through.create_new_celestia_blob(
					bcs::to_bytes(block)
					.map_err(|e| anyhow::anyhow!("Failed to serialize block: {}", e))?,
			)?;
			block_blobs.push(block_blob);
		}
		for block in blocks {
			info!(target: "movement_timing", block_id = %block.id(), "submitting_block");
		}
		self.pass_through.submit_celestia_blobs(&block_blobs).await?;
		for block in blocks {
			info!(target: "movement_timing", block_id = %block.id(), "submitted_block");
		}
		Ok(())
	}

	pub async fn submit_with_heuristic(&self, blocks : Vec<Block>) -> Result<(), anyhow::Error> {

		let mut heuristic : GroupingHeuristicStack<Block> = GroupingHeuristicStack::new(vec![
			DropSuccess::boxed(),
			ToApply::boxed(),
			SkipFor::boxed(1, Splitting::boxed(2)),
			FirstFitBinpacking::boxed(1_700_000_000)
		]);

		let _failed_blocks = heuristic.run_async_sequential(
			GroupingOutcome::new_apply_distribution(blocks),
			|grouping| async move {
				let blocks = grouping.into_original();
				match self.submit_blocks(&blocks).await {
					Ok(_) => Ok(GroupingOutcome::new_all_success(blocks.len())),
					Err(_) => Ok(GroupingOutcome::new_apply(blocks)),
				}
			},
		);

		Ok(())

	}

	/// Reads blobs from the receiver until the building time is exceeded
	async fn read_blocks(&self, receiver : &mut Receiver<Block>)-> Result<Vec<Block>, anyhow::Error> {
		let half_building_time = self.memseq.building_time_ms();
		let start = std::time::Instant::now();
		let mut blocks = Vec::new();
		loop {
			let remaining = match half_building_time.checked_sub(start.elapsed().as_millis() as u64) {
				Some(remaining) => remaining,
				None => {
					// we have exceeded the half building time
					break;
				}
			};
			match timeout(Duration::from_millis(remaining), receiver.recv()).await {
				Ok(Some(block)) => {
					// Process the block
					blocks.push(block);
				}
				Ok(None) => {
					// The channel was closed
					info!("sender dropped");
					break;
				}
				Err(_) => {
					// The operation timed out
					debug!(
						target: "movement_timing",
						batch_size = blocks.len(),
						"timed_out_building_block"
					);
					break;
				}
			}
		}
		Ok(blocks)
	}

	/// Ticks the block proposer to build blocks and submit them
	async fn tick_publish_blobs(&self, receiver : &mut Receiver<Block>) -> Result<(), anyhow::Error> {
		
		// get some blocks in a batch
		let blocks = self.read_blocks(receiver).await?;
		let ids = blocks.iter().map(|b| b.id()).collect::<Vec<_>>();

		// submit the blobs, resizing as needed
		for block_id in &ids {
			info!(target: "movement_timing", %block_id, "submitting_block_batch");
		}
		self.submit_with_heuristic(blocks).await?;
		for block_id in &ids {
			info!(target: "movement_timing", %block_id, "submitted_block_batch");
		}

		Ok(())
	}

	async fn run_block_builder(&self, sender : Sender<Block>) -> Result<(), anyhow::Error> {
		loop {
			self.tick_build_blocks(sender.clone()).await?;
		}
	}

	async fn run_block_publisher(&self, receiver : &mut Receiver<Block>) -> Result<(), anyhow::Error> {
		loop {
			self.tick_publish_blobs(receiver).await?;
		}
	}


	pub async fn run_block_proposer(&self) -> Result<(), anyhow::Error> {
		let (sender, mut receiver) = tokio::sync::mpsc::channel(2^10);

		loop {
			match futures::try_join!(
				self.run_block_builder(sender.clone()),
				self.run_block_publisher(&mut receiver),
			) {
				Ok(_) => {
					info!("block proposer completed");
				}
				Err(e) => {
					info!("block proposer failed: {:?}", e);
				}
			}	
		}

		Ok(())

	}

	pub fn to_sequenced_blob_block(
		blob_response: BlobResponse,
	) -> Result<BlobResponse, anyhow::Error> {
		let blob_type = blob_response.blob_type.ok_or(anyhow::anyhow!("No blob type"))?;

		let sequenced_block = match blob_type {
			blob_response::BlobType::PassedThroughBlob(blob) => {
				blob_response::BlobType::SequencedBlobBlock(blob)
			}
			blob_response::BlobType::SequencedBlobBlock(blob) => {
				blob_response::BlobType::SequencedBlobBlock(blob)
			}
			_ => {
				anyhow::bail!("Invalid blob type")
			}
		};

		Ok(BlobResponse { blob_type: Some(sequenced_block) })
	}

	pub fn make_sequenced_blob_intent(
		data: Vec<u8>,
		height: u64,
	) -> Result<BlobResponse, anyhow::Error> {
		Ok(BlobResponse {
			blob_type: Some(blob_response::BlobType::SequencedBlobIntent(Blob {
				data,
				blob_id: "".to_string(),
				height,
				timestamp: 0,
			})),
		})
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
		self.pass_through.stream_read_from_height(request).await
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
		self.pass_through.stream_read_latest(request).await
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
		unimplemented!("stream_write_blob")
	}
	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		request: tonic::Request<ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		self.pass_through.read_at_height(request).await
	}
	/// Batch read and write operations for efficiency.
	async fn batch_read(
		&self,
		request: tonic::Request<BatchReadRequest>,
	) -> std::result::Result<tonic::Response<BatchReadResponse>, tonic::Status> {
		self.pass_through.batch_read(request).await
	}

	/// Batch write blobs.
	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		let blobs_for_intent = request.into_inner().blobs;
		let blobs_for_submission = blobs_for_intent.clone();
		let height: u64 = self
			.pass_through
			.default_client
			.header_network_head()
			.await
			.map_err(|e| tonic::Status::internal(e.to_string()))?
			.height()
			.into();

		let intents: Vec<BlobResponse> = blobs_for_intent
			.into_iter()
			.map(|blob| {
				Self::make_sequenced_blob_intent(blob.data, height)
					.map_err(|e| tonic::Status::internal(e.to_string()))
			})
			.collect::<Result<Vec<BlobResponse>, tonic::Status>>()?;

		// make transactions from the blobs
		let mut transactions = Vec::new();
		for blob in blobs_for_submission {
			let transaction: Transaction = serde_json::from_slice(&blob.data)
				.map_err(|e| tonic::Status::internal(e.to_string()))?;
			transactions.push(transaction);
		}

		// publish the transactions
		let memseq = self.memseq.clone();
		memseq.publish_many(transactions).await.map_err(
			|e| tonic::Status::internal(e.to_string())
		)?;

		Ok(tonic::Response::new(BatchWriteResponse { blobs: intents }))
	}
	/// Update and manage verification parameters.
	async fn update_verification_parameters(
		&self,
		request: tonic::Request<UpdateVerificationParametersRequest>,
	) -> std::result::Result<tonic::Response<UpdateVerificationParametersResponse>, tonic::Status> {
		self.pass_through.update_verification_parameters(request).await
	}
}
