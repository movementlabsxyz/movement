use crate::SuzukaFullNode;
use m1_da_light_node_client::{
	blob_response, BatchWriteRequest, BlobWrite, LightNodeServiceClient,
	StreamReadFromHeightRequest,
};
use maptos_dof_execution::{
	v1::Executor, DynOptFinExecutor, ExecutableBlock, ExecutableTransactions, HashValue,
	SignatureVerifiedTransaction, SignedTransaction, Transaction,
};
use mcr_settlement_client::{McrSettlementClient, McrSettlementClientOperations};
use mcr_settlement_manager::CommitmentEventStream;
use mcr_settlement_manager::{McrSettlementManager, McrSettlementManagerOperations};
use movement_rest::MovementRest;
use movement_types::{Block, BlockCommitment, BlockCommitmentEvent};

use anyhow::Context;
use async_channel::{Receiver, Sender};
use core::sync::atomic::AtomicU64;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, info_span, warn, Instrument};
pub struct SuzukaPartialNode<T> {
	executor: T,
	transaction_sender: Sender<SignedTransaction>,
	pub transaction_receiver: Receiver<SignedTransaction>,
	light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
	settlement_manager: McrSettlementManager,
	movement_rest: MovementRest,
	pub config: suzuka_config::Config,
	da_db: Arc<DB>,
}

const LOGGING_UID: AtomicU64 = AtomicU64::new(0);

impl<T> SuzukaPartialNode<T>
where
	T: DynOptFinExecutor + Clone + Send + Sync,
{
	pub fn new<C>(
		executor: T,
		light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
		settlement_client: C,
		movement_rest: MovementRest,
		config: &suzuka_config::Config,
		da_db: DB,
	) -> (Self, impl Future<Output = Result<(), anyhow::Error>> + Send)
	where
		C: McrSettlementClientOperations + Send + 'static,
	{
		let (settlement_manager, commitment_events) =
			McrSettlementManager::new(settlement_client, &config.mcr);
		let (transaction_sender, transaction_receiver) = async_channel::unbounded();
		let bg_executor = executor.clone();
		(
			Self {
				executor,
				transaction_sender,
				transaction_receiver,
				light_node_client,
				settlement_manager,
				movement_rest,
				config: config.clone(),
				da_db: Arc::new(da_db),
			},
			read_commitment_events(commitment_events, bg_executor),
		)
	}

	fn bind_transaction_channel(&mut self) {
		self.executor.set_tx_channel(self.transaction_sender.clone());
	}

	pub fn bound<C>(
		executor: T,
		light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
		settlement_client: C,
		movement_rest: MovementRest,
		config: &suzuka_config::Config,
		da_db: DB,
	) -> Result<(Self, impl Future<Output = Result<(), anyhow::Error>> + Send), anyhow::Error>
	where
		C: McrSettlementClientOperations + Send + 'static,
	{
		let (mut node, background_task) =
			Self::new(executor, light_node_client, settlement_client, movement_rest, config, da_db);
		node.bind_transaction_channel();
		Ok((node, background_task))
	}

	async fn next_transaction_batch_write(&self) -> Result<(), anyhow::Error> {
		// limit the total time batching transactions
		let start = std::time::Instant::now();
		let (_, half_building_time) = self
			.config
			.m1_da_light_node
			.m1_da_light_node_config
			.try_block_building_parameters()?;

		let mut transactions = Vec::new();

		let batch_id = LOGGING_UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
		loop {
			let remaining = match half_building_time.checked_sub(start.elapsed().as_millis() as u64)
			{
				Some(remaining) => remaining,
				None => {
					// we have exceeded the half building time
					break;
				}
			};

			match tokio::time::timeout(
				Duration::from_millis(remaining),
				self.transaction_receiver.recv(),
			)
			.await
			{
				Ok(transaction) => match transaction {
					Ok(transaction) => {
						info!(
							target : "movement_timing",
							batch_id = %batch_id,
							tx_hash = %transaction.committed_hash(),
							sender = %transaction.sender(),
							sequence_number = transaction.sequence_number(),
							"received transaction",
						);
						let serialized_aptos_transaction = serde_json::to_vec(&transaction)?;
						let movement_transaction = movement_types::Transaction::new(
							serialized_aptos_transaction,
							transaction.sequence_number(),
						);
						let serialized_transaction = serde_json::to_vec(&movement_transaction)?;
						transactions.push(BlobWrite { data: serialized_transaction });
					}
					Err(_) => {
						break;
					}
				},
				Err(_) => {
					break;
				}
			}
		}

		if transactions.len() > 0 {
			info!(
				target: "movement_timing",
				batch_id = %batch_id,
				transaction_count = transactions.len(),
				"built_batch_write"
			);
			let batch_write = BatchWriteRequest { blobs: transactions };
			let mut light_node_client = self.light_node_client.clone();
			tokio::task::spawn(async move {
				light_node_client.batch_write(batch_write).await?;
				Ok::<(), anyhow::Error>(())
			});
		}

		Ok(())
	}

	async fn write_transactions_to_da(&self) -> Result<(), anyhow::Error> {
		loop {
			self.next_transaction_batch_write().await?;
		}
	}

	// receive transactions from the transaction channel and send them to be executed
	// ! This assumes the m1 da light node is running sequencer mode
	pub async fn read_blocks_from_da(&self) -> Result<(), anyhow::Error> {
		let mut stream = {
			let mut light_node_client = self.light_node_client.clone();
			light_node_client
				.stream_read_from_height(StreamReadFromHeightRequest {
					height: self.get_synced_height().await?,
				})
				.await?
		}
		.into_inner();

		while let Some(blob) = stream.next().await {
			debug!("Got blob: {:?}", blob);

			// get the block
			let (block_bytes, block_timestamp, block_id, da_height) = match blob?
				.blob
				.ok_or(anyhow::anyhow!("No blob in response"))?
				.blob_type
				.ok_or(anyhow::anyhow!("No blob type in response"))?
			{
				blob_response::BlobType::SequencedBlobBlock(blob) => {
					(blob.data, blob.timestamp, blob.blob_id, blob.height)
				}
				_ => {
					anyhow::bail!("Invalid blob type in response")
				}
			};

			// check if the block has already been executed
			if self.has_executed_block(block_id.clone()).await? {
				warn!("Block already executed: {:#?}. It will be skipped", block_id);
				continue;
			}

			// the da height must be greater than 1
			if da_height < 2 {
				anyhow::bail!("Invalid DA height: {:?}", da_height);
			}

			// decompress the block bytes
			let block = tokio::task::spawn_blocking(move || {
				let decompressed_block_bytes = zstd::decode_all(&block_bytes[..])?;
				let block: Block = bcs::from_bytes(&decompressed_block_bytes)?;
				Ok::<Block, anyhow::Error>(block)
			})
			.await??;

			// get the transactions
			let transaction_count = block.transactions.len();
			let span = info_span!(target: "movement_timing", "execute_block", id = %block_id);
			let commitment =
				self.execute_block_with_retries(block, block_timestamp).instrument(span).await?;
			self.executor.decrement_transactions_in_flight(transaction_count as u64);

			// mark the da_height - 1 as synced
			// we can't mark this height as synced because we must allow for the possibility of multiple blocks at the same height according to the m1 da specifications (which currently is built on celestia which itself allows more than one block at the same height)
			self.set_synced_height(da_height - 1).await?;

			// set the block as executed
			self.add_executed_block(block_id.to_string()).await?;

			// todo: this needs defaults
			if self.config.mcr.should_settle() {
				info!("Posting block commitment via settlement manager");
				match self.settlement_manager.post_block_commitment(commitment).await {
					Ok(_) => {}
					Err(e) => {
						error!("Failed to post block commitment: {:?}", e);
					}
				}
			} else {
				info!("Skipping settlement");
			}
		}

		Ok(())
	}

	/// Retries executing a block several times.
	/// This can be valid behavior if the block timestamps are too tightly clustered for the full node execution.
	/// However, this has to be deterministic, otherwise nodes will not be able to agree on the block commitment.
	///
	/// This protocol has a bit of a cascading effect, whereby increasing the timestamp of a block will mean that the next block has a greater likelihood of also needing to have its timestamp increased and with a greater number of retries. This will generally reset so long as the retry increment and count do not increases the timestamp beyond the block building time.
	async fn execute_block_with_retries(
		&self,
		block: Block,
		mut block_timestamp: u64,
	) -> anyhow::Result<BlockCommitment> {
		for _ in 0..self.config.execution_extension.block_retry_count {
			// we have to clone here because the block is supposed to be consumed by the executor
			match self.execute_block(block.clone(), block_timestamp).await {
				Ok(commitment) => return Ok(commitment),
				Err(e) => {
					error!("Failed to execute block: {:?}. Retrying", e);
					block_timestamp +=
						self.config.execution_extension.block_retry_increment_microseconds; // increase the timestamp by 5 ms (5000 microseconds)
				}
			}
		}

		anyhow::bail!("Failed to execute block after 5 retries")
	}

	async fn execute_block(
		&self,
		block: Block,
		block_timestamp: u64,
	) -> anyhow::Result<BlockCommitment> {
		let block_id = block.id();
		let block_hash = HashValue::from_slice(block.id())?;

		// get the transactions
		let mut block_transactions = Vec::new();
		let block_metadata = self
			.executor
			.build_block_metadata(HashValue::sha3_256_of(block_id.0.as_slice()), block_timestamp)
			.await?;
		let block_metadata_transaction =
			SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(block_metadata));
		block_transactions.push(block_metadata_transaction);

		for transaction in block.transactions {
			let signed_transaction: SignedTransaction = serde_json::from_slice(&transaction.data)?;
			let signature_verified_transaction = SignatureVerifiedTransaction::Valid(
				Transaction::UserTransaction(signed_transaction),
			);
			block_transactions.push(signature_verified_transaction);
		}

		// form the executable transactions vec
		let block = ExecutableTransactions::Unsharded(block_transactions);

		// form the executable block and execute it
		let executable_block = ExecutableBlock::new(block_hash, block);
		let block_id = executable_block.block_id;
		let commitment = self.executor.execute_block_opt(executable_block).await?;

		info!("Executed block: {}", block_id);

		Ok(commitment)
	}
}

pub async fn read_commitment_events<T>(
	mut stream: CommitmentEventStream,
	executor: T,
) -> anyhow::Result<()>
where
	T: DynOptFinExecutor + Send + Sync,
{
	while let Some(res) = stream.next().await {
		let event = match res {
			Ok(event) => event,
			Err(e) => {
				error!("Failed to get commitment event: {:?}", e);
				continue;
			}
		};
		match event {
			BlockCommitmentEvent::Accepted(commitment) => {
				debug!("Commitment accepted: {:?}", commitment);
				match executor.set_finalized_block_height(commitment.height) {
					Ok(_) => {}
					Err(e) => {
						error!("Failed to set finalized block height: {:?}", e);
					}
				}
			}
			BlockCommitmentEvent::Rejected { height, reason } => {
				debug!("Commitment rejected: {:?} {:?}", height, reason);
				// TODO: block reversion
			}
		}
	}

	Ok(())
}

impl<T> SuzukaFullNode for SuzukaPartialNode<T>
where
	T: DynOptFinExecutor + Clone + Send + Sync,
{
	/// Runs the services until crash or shutdown.
	async fn run_services(&self) -> Result<(), anyhow::Error> {
		self.executor.run_service().await?;

		Ok(())
	}

	/// Runs the background tasks until crash or shutdown.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		self.executor.run_background_tasks().await?;

		Ok(())
	}

	// ! Currently this only implements opt.
	/// Runs the executor until crash or shutdown.
	async fn run_executor(&self) -> Result<(), anyhow::Error> {
		// ! todo: this is a temporary solution to rollover the genesis block, really this (a) needs to be read from the DA and (b) requires modifications to Aptos Core.
		self.executor.rollover_genesis_block().await?;
		// wait for both tasks to finish
		tokio::try_join!(self.write_transactions_to_da(), self.read_blocks_from_da())?;

		Ok(())
	}

	/// Runs the maptos rest api service until crash or shutdown.
	async fn run_movement_rest(&self) -> Result<(), anyhow::Error> {
		self.movement_rest.run_service().await?;
		Ok(())
	}
}

impl<T> SuzukaPartialNode<T> {
	pub async fn create_or_get_da_db(config: &suzuka_config::Config) -> Result<DB, anyhow::Error> {
		let path = config.da_db.da_db_path.clone();

		let mut options = Options::default();
		options.create_if_missing(true);
		options.create_missing_column_families(true);

		let synced_height = ColumnFamilyDescriptor::new("synced_height", Options::default());
		let executed_blocks = ColumnFamilyDescriptor::new("executed_blocks", Options::default());

		let db = DB::open_cf_descriptors(&options, path, vec![synced_height, executed_blocks])
			.map_err(|e| anyhow::anyhow!("Failed to open DA DB: {:?}", e))?;

		Ok(db)
	}

	pub async fn set_synced_height(&self, height: u64) -> Result<(), anyhow::Error> {
		// This is heavy for this purpose, but progressively the contents of the DA DB will be used for more things
		let da_db = self.da_db.clone();
		tokio::task::spawn_blocking(move || {
			let cf = da_db
				.cf_handle("synced_height")
				.ok_or(anyhow::anyhow!("No synced_height column family"))?;
			let height = serde_json::to_string(&height)
				.map_err(|e| anyhow::anyhow!("Failed to serialize synced height: {:?}", e))?;
			da_db
				.put_cf(&cf, "synced_height", height)
				.map_err(|e| anyhow::anyhow!("Failed to set synced height: {:?}", e))
		})
		.await??;
		Ok(())
	}

	pub async fn get_synced_height(&self) -> Result<u64, anyhow::Error> {
		// This is heavy for this purpose, but progressively the contents of the DA DB will be used for more things
		let da_db = self.da_db.clone();
		let height = tokio::task::spawn_blocking(move || {
			let cf = da_db
				.cf_handle("synced_height")
				.ok_or(anyhow::anyhow!("No synced_height column family"))?;
			let height = da_db
				.get_cf(&cf, "synced_height")
				.map_err(|e| anyhow::anyhow!("Failed to get synced height: {:?}", e))?;
			let height = match height {
				Some(height) => serde_json::from_slice(&height)
					.map_err(|e| anyhow::anyhow!("Failed to deserialize synced height: {:?}", e))?,
				None => 0,
			};
			Ok::<u64, anyhow::Error>(height)
		})
		.await??;
		Ok(height)
	}

	pub async fn add_executed_block(&self, id: String) -> Result<(), anyhow::Error> {
		let da_db = self.da_db.clone();
		tokio::task::spawn_blocking(move || {
			let cf = da_db
				.cf_handle("executed_blocks")
				.ok_or(anyhow::anyhow!("No executed_blocks column family"))?;
			da_db
				.put_cf(&cf, id.clone(), id)
				.map_err(|e| anyhow::anyhow!("Failed to add executed block: {:?}", e))
		})
		.await??;
		Ok(())
	}

	pub async fn has_executed_block(&self, id: String) -> Result<bool, anyhow::Error> {
		let da_db = self.da_db.clone();
		let id = tokio::task::spawn_blocking(move || {
			let cf = da_db
				.cf_handle("executed_blocks")
				.ok_or(anyhow::anyhow!("No executed_blocks column family"))?;
			da_db
				.get_cf(&cf, id)
				.map_err(|e| anyhow::anyhow!("Failed to get executed block: {:?}", e))
		})
		.await??;
		Ok(id.is_some())
	}
}

impl SuzukaPartialNode<Executor> {
	pub async fn try_from_config(
		config: suzuka_config::Config,
	) -> Result<(Self, impl Future<Output = Result<(), anyhow::Error>> + Send), anyhow::Error> {
		let (tx, _) = async_channel::unbounded();

		// todo: extract into getter
		let light_node_connection_hostname = config
			.m1_da_light_node
			.m1_da_light_node_config
			.m1_da_light_node_connection_hostname();

		// todo: extract into getter
		let light_node_connection_port = config
			.m1_da_light_node
			.m1_da_light_node_config
			.m1_da_light_node_connection_port();
		// todo: extract into getter
		debug!(
			"Connecting to light node at {}:{}",
			light_node_connection_hostname, light_node_connection_port
		);
		let light_node_client = LightNodeServiceClient::connect(format!(
			"http://{}:{}",
			light_node_connection_hostname, light_node_connection_port
		))
		.await
		.context("Failed to connect to light node")?;

		debug!("Creating the executor");
		let executor = Executor::try_from_config(tx, config.execution_config.maptos_config.clone())
			.context("Failed to create the inner executor")?;

		debug!("Creating the settlement client");
		let settlement_client = McrSettlementClient::build_with_config(config.mcr.clone())
			.await
			.context("Failed to build MCR settlement client with config")?;

		debug!("Creating the movement rest service");
		let movement_rest = MovementRest::try_from_env(Some(executor.executor.context.clone()))
			.context("Failed to create MovementRest")?;

		debug!("Creating the DA DB");
		let da_db = Self::create_or_get_da_db(&config)
			.await
			.context("Failed to create or get DA DB")?;

		Self::bound(executor, light_node_client, settlement_client, movement_rest, &config, da_db)
			.context(
			"Failed to bind the executor, light node client, settlement client, and movement rest",
		)
	}
}
