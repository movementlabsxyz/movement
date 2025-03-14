use futures::future::join_all;
use movement_da_light_node_da::DaOperations;
use movement_da_light_node_prevalidator::{aptos::whitelist::Validator, PrevalidatorOperations};
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer_loader::LoadedSigner;
use std::boxed::Box;
use std::fmt::Debug;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{atomic::AtomicU64, Arc};
use std::time::Duration;

use memseq::{Sequencer, Transaction};
use movement_da_light_node_celestia::da::Da as CelestiaDa;
use movement_da_light_node_digest_store::da::Da as DigestStoreDa;
use movement_da_light_node_proto as grpc;
use movement_da_light_node_proto::blob_response::BlobType;
use movement_da_light_node_proto::light_node_service_server::LightNodeService;
use movement_da_light_node_verifier::{signed::InKnownSignersVerifier, VerifierOperations};
use movement_da_util::{
	blob::ir::{blob::DaBlob, data::InnerSignedBlobV1Data},
	config::Config,
};
use movement_signer::{cryptography::Curve, Digester, Signing, Verify};
use movement_types::block::Block;
use serde::{Deserialize, Serialize};
use tokio::{
	sync::mpsc::{Receiver, Sender},
	time::timeout,
};
use tokio_stream::Stream;
use tracing::{debug, info};

use crate::{passthrough::LightNode as LightNodePassThrough, LightNodeRuntime};

const LOGGING_UID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct LightNode<O, C, Da, V>
where
	O: Signing<C> + Send + Sync + Clone,
	C: Curve
		+ Verify<C>
		+ Digester<C>
		+ Send
		+ Sync
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ Clone
		+ 'static
		+ std::fmt::Debug,
	Da: DaOperations<C>,
	V: VerifierOperations<DaBlob<C>, DaBlob<C>>,
{
	pub pass_through: LightNodePassThrough<O, C, Da, V>,
	pub memseq: Arc<memseq::Memseq<memseq::RocksdbMempool>>,
	pub prevalidator: Option<Arc<Validator>>,
}

impl<O, C, Da, V> Debug for LightNode<O, C, Da, V>
where
	O: Signing<C> + Send + Sync + Clone,
	C: Curve
		+ Verify<C>
		+ Digester<C>
		+ Send
		+ Sync
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ Clone
		+ 'static
		+ std::fmt::Debug,
	Da: DaOperations<C>,
	V: VerifierOperations<DaBlob<C>, DaBlob<C>>,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("LightNode").field("pass_through", &self.pass_through).finish()
	}
}

impl LightNodeRuntime
	for LightNode<
		LoadedSigner<Secp256k1>,
		Secp256k1,
		DigestStoreDa<Secp256k1, CelestiaDa<Secp256k1>>,
		InKnownSignersVerifier<Secp256k1>,
	>
{
	async fn try_from_config(config: Config) -> Result<Self, anyhow::Error> {
		info!("Initializing LightNode in sequencer mode from environment.");

		let pass_through = LightNodePassThrough::try_from_config(config.clone()).await?;
		info!("Initialized pass through for LightNode in sequencer mode: {pass_through:?}");

		let memseq_path = pass_through.config.try_memseq_path()?;
		info!("Memseq path: {:?}", memseq_path);
		let (max_block_size, build_time) = pass_through.config.block_building_parameters();

		let memseq = Arc::new(memseq::Memseq::try_move_rocks(
			PathBuf::from(memseq_path),
			max_block_size,
			build_time,
		)?);
		info!("Initialized Memseq with Move Rocks for LightNode in sequencer mode.");

		// prevalidator
		let whitelisted_accounts = config.whitelisted_accounts()?;
		let prevalidator = match whitelisted_accounts {
			Some(whitelisted_accounts) => Some(Arc::new(Validator::new(whitelisted_accounts))),
			None => None,
		};

		Ok(Self { pass_through, memseq, prevalidator })
	}

	fn try_service_address(&self) -> Result<String, anyhow::Error> {
		self.pass_through.try_service_address()
	}

	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		self.run_block_proposer().await?;

		Ok(())
	}
}

impl<O, C, Da, V> LightNode<O, C, Da, V>
where
	O: Signing<C> + Send + Sync + Clone,
	C: Curve
		+ Verify<C>
		+ Digester<C>
		+ Send
		+ Sync
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ Clone
		+ 'static
		+ std::fmt::Debug,
	Da: DaOperations<C>,
	V: VerifierOperations<DaBlob<C>, DaBlob<C>>,
{
	async fn tick_build_blocks(&self, sender: Sender<Block>) -> Result<(), anyhow::Error> {
		info!(target: "movement_timing", "tick_build_blocks");
		let memseq = self.memseq.clone();

		// this has an internal timeout based on its building time
		// so in the worst case scenario we will roughly double the internal timeout
		let uid = LOGGING_UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		info!(target: "movement_timing", tick = &uid, "CALLED tick_build_blocks tick");
		let block = memseq.wait_for_next_block().await?;
		match block {
			Some(block) => {
				info!(target: "movement_timing", block_id = %block.id(), uid = %uid, transaction_count = block.transactions().len(), "tick_build_blocks: received_block");
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

	/// Submits blocks to the pass through.
	///
	/// Note: if you change the block submission architecture to an eventually consistent submission pattern, you will want to revert this to a sequential submission.
	/// Currently, all of the blobs can just be sent through because we are relying on Celestia for order.
	async fn submit_blocks(&self, blocks: Vec<Block>) -> Result<(), anyhow::Error> {
		let futures = blocks.into_iter().map(|block| async {
			let data: InnerSignedBlobV1Data<C> = block.try_into()?;
			let blob = data.try_to_sign(&self.pass_through.signer).await?;
			self.pass_through.da.submit_blob(blob.into()).await?;
			Ok::<(), anyhow::Error>(())
		});

		join_all(futures).await.into_iter().collect::<Result<(), _>>()?;
		Ok(())
	}

	/// Collapses the back-pressured blocks into a single block and submits it.
	///
	/// todo: mark not pub once this is actually used
	pub async fn submit_collapsed_blocks(&self, blocks: Vec<Block>) -> Result<(), anyhow::Error> {
		let block = Block::collapse(blocks);
		let data: InnerSignedBlobV1Data<C> = block.try_into()?;
		let blob = data.try_to_sign(&self.pass_through.signer).await?;
		self.pass_through.da.submit_blob(blob.into()).await?;
		Ok(())
	}

	/// Reads blobs from the receiver until the building time is exceeded
	async fn read_blocks(
		&self,
		receiver: &mut Receiver<Block>,
	) -> Result<Vec<Block>, anyhow::Error> {
		let half_building_time = self.memseq.building_time_ms();
		let start = std::time::Instant::now();
		let mut blocks = Vec::new();
		loop {
			let remaining = match half_building_time.checked_sub(start.elapsed().as_millis() as u64)
			{
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

		info!(target: "movement_timing", block_count = blocks.len(), "read_blocks");

		Ok(blocks)
	}

	/// Ticks the block proposer to build blocks and submit them
	async fn tick_publish_blobs(
		&self,
		receiver: &mut Receiver<Block>,
	) -> Result<(), anyhow::Error> {
		// get some blocks in a batch
		let blocks = self.read_blocks(receiver).await?;
		if blocks.is_empty() {
			return Ok(());
		}

		// submit the blobs, resizing as needed
		let ids = blocks.iter().map(|b| b.id()).collect::<Vec<_>>();
		for block_id in &ids {
			info!(target: "movement_timing", %block_id, "submitting_block_batch");
		}
		self.submit_blocks(blocks).await?;
		for block_id in &ids {
			info!(target: "movement_timing", %block_id, "submitted_block_batch");
		}

		Ok(())
	}

	async fn run_block_builder(&self, sender: Sender<Block>) -> Result<(), anyhow::Error> {
		loop {
			self.tick_build_blocks(sender.clone()).await?;
		}
	}

	async fn run_block_publisher(
		&self,
		receiver: &mut Receiver<Block>,
	) -> Result<(), anyhow::Error> {
		loop {
			self.tick_publish_blobs(receiver).await?;
		}
	}

	// FIXME: this does not work correctly, see details in move-rocks
	#[allow(dead_code)]
	async fn run_gc(&self) -> Result<(), anyhow::Error> {
		loop {
			self.memseq.gc().await?;
		}
	}

	pub async fn run_block_proposer(&self) -> Result<(), anyhow::Error> {
		let (sender, mut receiver) = tokio::sync::mpsc::channel(2 ^ 10);

		loop {
			info!(target: "movement_timing", "START: run_block_propoer iteration");
			match futures::try_join!(
				self.run_block_builder(sender.clone()),
				self.run_block_publisher(&mut receiver),
				// self.run_gc(),
			) {
				Ok(_) => {
					info!("block proposer completed");
				}
				Err(e) => {
					info!("block proposer failed: {:?}", e);
					return Err(e);
				}
			}
		}
	}

	pub fn to_sequenced_blob_block(
		blob_response: grpc::BlobResponse,
	) -> Result<grpc::BlobResponse, anyhow::Error> {
		let blob_type = blob_response.blob_type.ok_or(anyhow::anyhow!("No blob type"))?;

		let sequenced_block = match blob_type {
			BlobType::PassedThroughBlob(blob) => BlobType::SequencedBlobBlock(blob),
			BlobType::SequencedBlobBlock(blob) => BlobType::SequencedBlobBlock(blob),
			_ => {
				anyhow::bail!("Invalid blob type")
			}
		};

		Ok(grpc::BlobResponse { blob_type: Some(sequenced_block) })
	}
}

#[tonic::async_trait]
impl<O, C, Da, V> LightNodeService for LightNode<O, C, Da, V>
where
	O: Signing<C> + Send + Sync + Clone + 'static,
	C: Curve
		+ Verify<C>
		+ Digester<C>
		+ Send
		+ Sync
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ Clone
		+ 'static
		+ std::fmt::Debug,
	Da: DaOperations<C> + 'static,
	V: VerifierOperations<DaBlob<C>, DaBlob<C>> + Send + Sync + 'static,
{
	/// Server streaming response type for the StreamReadFromHeight method.
	type StreamReadFromHeightStream = Pin<
		Box<
			dyn Stream<Item = Result<grpc::StreamReadFromHeightResponse, tonic::Status>>
				+ Send
				+ 'static,
		>,
	>;

	/// Stream blobs from a specified height or from the latest height.
	async fn stream_read_from_height(
		&self,
		request: tonic::Request<grpc::StreamReadFromHeightRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadFromHeightStream>, tonic::Status> {
		self.pass_through.stream_read_from_height(request).await
	}

	/// Server streaming response type for the StreamReadLatest method.
	type StreamReadLatestStream = Pin<
		Box<
			dyn Stream<Item = Result<grpc::StreamReadLatestResponse, tonic::Status>>
				+ Send
				+ 'static,
		>,
	>;

	/// Stream the latest blobs.
	async fn stream_read_latest(
		&self,
		request: tonic::Request<grpc::StreamReadLatestRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadLatestStream>, tonic::Status> {
		self.pass_through.stream_read_latest(request).await
	}
	/// Server streaming response type for the StreamWriteCelestiaBlob method.
	type StreamWriteBlobStream = Pin<
		Box<
			dyn Stream<Item = Result<grpc::StreamWriteBlobResponse, tonic::Status>>
				+ Send
				+ 'static,
		>,
	>;
	/// Stream blobs out, either individually or in batches.
	async fn stream_write_blob(
		&self,
		_request: tonic::Request<tonic::Streaming<grpc::StreamWriteBlobRequest>>,
	) -> std::result::Result<tonic::Response<Self::StreamWriteBlobStream>, tonic::Status> {
		unimplemented!("stream_write_blob")
	}
	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		request: tonic::Request<grpc::ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<grpc::ReadAtHeightResponse>, tonic::Status> {
		self.pass_through.read_at_height(request).await
	}
	/// Batch read and write operations for efficiency.
	async fn batch_read(
		&self,
		request: tonic::Request<grpc::BatchReadRequest>,
	) -> std::result::Result<tonic::Response<grpc::BatchReadResponse>, tonic::Status> {
		self.pass_through.batch_read(request).await
	}

	/// Batch write blobs.
	async fn batch_write(
		&self,
		request: tonic::Request<grpc::BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<grpc::BatchWriteResponse>, tonic::Status> {
		let blobs_for_submission = request.into_inner().blobs;

		// make transactions from the blobs
		let mut transactions = Vec::new();
		for blob in blobs_for_submission {
			let transaction: Transaction = serde_json::from_slice(&blob.data)
				.map_err(|e| tonic::Status::internal(e.to_string()))?;

			match &self.prevalidator {
				Some(prevalidator) => {
					// match the prevalidated status, if validation error discard if internal error raise internal error
					match prevalidator.prevalidate(transaction).await {
						Ok(prevalidated) => {
							transactions.push(prevalidated.into_inner());
						}
						Err(e) => {
							match e {
								movement_da_light_node_prevalidator::Error::Validation(_) => {
									// discard the transaction
									info!(
										"discarding transaction due to prevalidation error {:?}",
										e
									);
								}
								movement_da_light_node_prevalidator::Error::Internal(e) => {
									return Err(tonic::Status::internal(e.to_string()));
								}
							}
						}
					}
				}
				None => transactions.push(transaction),
			}
		}

		// publish the transactions
		let memseq = self.memseq.clone();
		memseq
			.publish_many(transactions)
			.await
			.map_err(|e| tonic::Status::internal(e.to_string()))?;

		Ok(tonic::Response::new(grpc::BatchWriteResponse { blobs: vec![] }))
	}
}
