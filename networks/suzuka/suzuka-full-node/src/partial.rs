use crate::{da_db::DaDB, tasks};
use m1_da_light_node_client::LightNodeServiceClient;
use maptos_dof_execution::MakeOptFinServices;
use maptos_dof_execution::{v1::Executor, DynOptFinExecutor};
use mcr_settlement_client::McrSettlementClient;
use mcr_settlement_manager::CommitmentEventStream;
use mcr_settlement_manager::McrSettlementManager;
use movement_rest::MovementRest;
use suzuka_config::Config;

use anyhow::Context;
use tokio::sync::mpsc;
use tokio::try_join;
use tracing::debug;

pub struct SuzukaPartialNode<T> {
	executor: T,
	light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
	settlement_manager: McrSettlementManager,
	commitment_events: Option<CommitmentEventStream>,
	movement_rest: MovementRest,
	config: Config,
	da_db: DaDB,
}

impl<T> SuzukaPartialNode<T>
where
	T: DynOptFinExecutor + Send + 'static,
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
	async fn execute_block_with_retries(
		&self,
		block: Block,
		mut block_timestamp: u64,
	) -> anyhow::Result<BlockCommitment> {
		for _ in 0..5 {
			// we have to clone here because the block is supposed to be consumed by the executor
			match self.execute_block(block.clone(), block_timestamp).await {
				Ok(commitment) => return Ok(commitment),
				Err(e) => {
					error!("Failed to execute block: {:?}. Retrying", e);
					block_timestamp += 5000; // increase the timestamp by 5 ms (5000 microseconds)
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
				let current_head_height = executor.get_block_head_height().await?;
				if height > current_head_height {
					// Nothing to revert
				} else {
					match executor.revert_block_head_to(height - 1) {
						Ok(_) => {}
						Err(e) => {
							error!("Failed to revert to block height: {:?}", e);
						}
					}
				}
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
	pub async fn run(self) -> Result<(), anyhow::Error> {
		let (transaction_sender, transaction_receiver) = mpsc::channel(16);
		let (context, exec_background) = self
			.executor
			.background(transaction_sender, &self.config.execution_config.maptos_config)?;
		let services = context.services();
		let mut movement_rest = self.movement_rest;
		movement_rest.set_context(services.opt_api_context());
		let exec_settle_task = tasks::execute_settle::Task::new(
			self.executor,
			self.settlement_manager,
			self.da_db,
			self.light_node_client.clone(),
			self.commitment_events,
			self.config.execution_extension.clone(),
		);
		let transaction_ingress_task = tasks::transaction_ingress::Task::new(
			transaction_receiver,
			self.light_node_client,
			// FIXME: why are the struct member names so tautological?
			self.config.m1_da_light_node.m1_da_light_node_config,
		);

		let (
			execution_and_settlement_result,
			transaction_ingress_result,
			background_task_result,
			services_result,
		) = try_join!(
			tokio::spawn(async move { exec_settle_task.run().await }),
			tokio::spawn(async move { transaction_ingress_task.run().await }),
			tokio::spawn(exec_background),
			tokio::spawn(services.run()),
			// tokio::spawn(async move { movement_rest.run_service().await }),
		)?;
		execution_and_settlement_result
			.and(transaction_ingress_result)
			.and(background_task_result)
			.and(services_result)
	}
}

impl SuzukaPartialNode<Executor> {
	pub async fn try_from_config(config: Config) -> Result<Self, anyhow::Error> {
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
		let executor = Executor::try_from_config(&config.execution_config.maptos_config)
			.context("Failed to create the inner executor")?;

		debug!("Creating the settlement client");
		let settlement_client = McrSettlementClient::build_with_config(&config.mcr)
			.await
			.context("Failed to build MCR settlement client with config")?;
		let (settlement_manager, commitment_events) =
			McrSettlementManager::new(settlement_client, &config.mcr);
		let commitment_events =
			if config.mcr.should_settle() { Some(commitment_events) } else { None };

		debug!("Creating the movement rest service");
		let movement_rest =
			MovementRest::try_from_env().context("Failed to create MovementRest")?;

		debug!("Creating the DA DB");
		let da_db =
			DaDB::open(&config.da_db.da_db_path).context("Failed to create or get DA DB")?;

		Ok(Self {
			executor,
			light_node_client,
			settlement_manager,
			commitment_events,
			movement_rest,
			config,
			da_db,
		})
	}
}
