use crate::SuzukaFullNode;
use m1_da_light_node_client::{
	blob_response, BatchWriteRequest, BlobWrite, LightNodeServiceClient,
	StreamReadFromHeightRequest,
};
use maptos_dof_execution::{
	v1::Executor, DynOptFinExecutor, ExecutableBlock, ExecutableTransactions, HashValue,
	SignatureVerifiedTransaction, SignedTransaction, Transaction,
};
use mcr_settlement_client::{
 McrSettlementClient, McrSettlementClientOperations,
};
use mcr_settlement_manager::CommitmentEventStream;
use mcr_settlement_manager::{McrSettlementManager, McrSettlementManagerOperations};
use movement_rest::MovementRest;
use movement_types::{Block, BlockCommitmentEvent};

use anyhow::Context;
use async_channel::{Receiver, Sender};
use sha2::Digest;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::{debug, info, error};

use std::future::{self, Future};
use std::sync::Arc;
use std::time::Duration;
pub struct SuzukaPartialNode<T> {
	executor: T,
	transaction_sender: Sender<SignedTransaction>,
	pub transaction_receiver: Receiver<SignedTransaction>,
	light_node_client: Arc<RwLock<LightNodeServiceClient<tonic::transport::Channel>>>,
	pub settlement_manager: McrSettlementManager,
	movement_rest: MovementRest,
	pub config: suzuka_config::Config,
}

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
				light_node_client: Arc::new(RwLock::new(light_node_client)),
				settlement_manager,
				movement_rest,
				config: config.clone(),
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
	) -> Result<(Self, impl Future<Output = Result<(), anyhow::Error>> + Send), anyhow::Error>
	where
		C: McrSettlementClientOperations + Send + 'static,
	{
		let (mut node, background_task) =
			Self::new(executor, light_node_client, settlement_client, movement_rest, config);
		node.bind_transaction_channel();
		Ok((node, background_task))
	}

	pub async fn tick_write_transactions_to_da(&self) -> Result<(), anyhow::Error> {
		// limit the total time batching transactions
		let start_time = std::time::Instant::now();
		let end_time = start_time + std::time::Duration::from_millis(100);

		let mut transactions = Vec::new();

		while let Ok(transaction_result) =
			tokio::time::timeout(Duration::from_millis(100), self.transaction_receiver.recv()).await
		{
			match transaction_result {
				Ok(transaction) => {
					debug!("Got transaction: {:?}", transaction);

					let serialized_aptos_transaction = serde_json::to_vec(&transaction)?;
					debug!("Serialized transaction: {:?}", serialized_aptos_transaction);
					let movement_transaction = movement_types::Transaction {
						data : serialized_aptos_transaction,
						sequence_number : transaction.sequence_number()
					};
					let serialized_transaction = serde_json::to_vec(&movement_transaction)?;
					transactions.push(BlobWrite { data: serialized_transaction });
				}
				Err(_) => {
					break;
				}
			}

			if std::time::Instant::now() > end_time {
				break;
			}
		}

		if transactions.len() > 0 {
			let client_ptr = self.light_node_client.clone();
			let mut light_node_client = client_ptr.write().await;
			light_node_client.batch_write(BatchWriteRequest { blobs: transactions }).await?;

			debug!("Wrote transactions to DA");
		}

		Ok(())
	}

	pub async fn write_transactions_to_da(&self) -> Result<(), anyhow::Error> {
		loop {
			self.tick_write_transactions_to_da().await?;
		}
	}

	// receive transactions from the transaction channel and send them to be executed
	// ! This assumes the m1 da light node is running sequencer mode
	pub async fn read_blocks_from_da(&self) -> Result<(), anyhow::Error> {
		let block_head_height = self.executor.get_block_head_height().await?;

		let mut stream = {
			let client_ptr = self.light_node_client.clone();
			let mut light_node_client = client_ptr.write().await;
			light_node_client
				.stream_read_from_height(StreamReadFromHeightRequest { height: block_head_height })
				.await?
		}
		.into_inner();

		while let Some(blob) = stream.next().await {
			debug!("Got blob: {:?}", blob);

			// get the block
			let (block_bytes, block_timestamp, block_id) = match blob?
				.blob
				.ok_or(anyhow::anyhow!("No blob in response"))?
				.blob_type
				.ok_or(anyhow::anyhow!("No blob type in response"))?
			{
				blob_response::BlobType::SequencedBlobBlock(blob) => {
					(blob.data, blob.timestamp, blob.blob_id)
				}
				_ => {
					anyhow::bail!("Invalid blob type in response")
				}
			};

			let block: Block = serde_json::from_slice(&block_bytes)?;

			debug!("Got block: {:?}", block);
			info!("Block micros timestamp: {:?}", block_timestamp);

			// get the transactions
			let mut block_transactions = Vec::new();
			let block_metadata = self
				.executor
				.build_block_metadata(HashValue::sha3_256_of(block_id.as_bytes()), block_timestamp)
				.await?;
			let block_metadata_transaction =
				SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(block_metadata));
			block_transactions.push(block_metadata_transaction);

			for transaction in block.transactions {
				let signed_transaction = serde_json::from_slice(&transaction.data)?;
				let signature_verified_transaction = SignatureVerifiedTransaction::Valid(
					Transaction::UserTransaction(signed_transaction),
				);
				block_transactions.push(signature_verified_transaction);
			}

			// form the executable transactions vec
			let block = ExecutableTransactions::Unsharded(block_transactions);

			// hash the block bytes
			let mut hasher = sha2::Sha256::new();
			hasher.update(&block_bytes);
			let slice = hasher.finalize();
			let block_hash = HashValue::from_slice(slice.as_slice())?;

			// form the executable block and execute it
			let executable_block = ExecutableBlock::new(block_hash, block);
			let block_id = executable_block.block_id;
			let commitment = self.executor.execute_block_opt(executable_block).await?;
			info!("Executed block: {:?}", block_id);

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

	Ok(future::pending().await)
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

impl SuzukaPartialNode<Executor> {
	pub async fn try_from_config(
		config: suzuka_config::Config,
	) -> Result<(Self, impl Future<Output = Result<(), anyhow::Error>> + Send), anyhow::Error> {
		let (tx, _) = async_channel::unbounded();

		// todo: extract into getter
		let light_node_connection_hostname = match &config.m1_da_light_node.m1_da_light_node_config
		{
			m1_da_light_node_util::config::Config::Local(local) => {
				local.m1_da_light_node.m1_da_light_node_connection_hostname.clone()
			}
		};

		// todo: extract into getter
		let light_node_connection_port = match &config.m1_da_light_node.m1_da_light_node_config {
			m1_da_light_node_util::config::Config::Local(local) => {
				local.m1_da_light_node.m1_da_light_node_connection_port.clone()
			}
		};

		// todo: extract into getter
		debug!("Connecting to light node at {}:{}", light_node_connection_hostname, light_node_connection_port);
		let light_node_client = LightNodeServiceClient::connect(format!(
			"http://{}:{}",
			light_node_connection_hostname, light_node_connection_port
		))
		.await.context("Failed to connect to light node")?;

		debug!("Creating the executor");
		let executor = Executor::try_from_config(tx, config.execution_config.maptos_config.clone())
			.context("Failed to create the inner executor")?;

		debug!("Creating the settlement client");
		let settlement_client =
			McrSettlementClient::build_with_config(config.mcr.clone()).await.context(
				"Failed to build MCR settlement client with config",
			)?;

		debug!("Creating the movement rest service");
		let movement_rest = MovementRest::try_from_env(Some(executor.executor.context.clone())).context("Failed to create MovementRest")?;

		Self::bound(executor, light_node_client, settlement_client, movement_rest, &config).context(
			"Failed to bind the executor, light node client, settlement client, and movement rest"
		)
		
	}
}
