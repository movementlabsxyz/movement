//! Task module to execute blocks from the DA and process settlement.
use crate::node::da_db::DaDB;
use anyhow::Context;
use futures::{future::Either, stream};
use maptos_dof_execution::{
	DynOptFinExecutor, ExecutableBlock, ExecutableTransactions, HashValue,
	SignatureVerifiedTransaction, SignedTransaction, Transaction,
};
use mcr_settlement_manager::{CommitmentEventStream, McrSettlementManagerOperations};
use movement_config::execution_extension;
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_client::GrpcDaSequencerClient;
use movement_da_sequencer_proto::BlockV1;
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use movement_types::block::{Block, BlockCommitment, BlockCommitmentEvent};
use tokio::select;
use tokio_stream::{Stream, StreamExt};
use tracing::{debug, error, info, info_span, Instrument};
use url::Url;

pub struct Task<E, S> {
	executor: E,
	settlement_manager: Option<S>,
	da_db: DaDB,
	// Stream receiving commitment events, conditionally enabled
	commitment_events:
		Either<CommitmentEventStream, stream::Pending<<CommitmentEventStream as Stream>::Item>>,
	execution_extension: execution_extension::Config,
	settlement_config: mcr_settlement_config::Config,
}

impl<E, S> Task<E, S> {
	pub(crate) fn new(
		executor: E,
		settlement_manager: Option<S>,
		da_db: DaDB,
		commitment_events: Option<CommitmentEventStream>,
		execution_extension: execution_extension::Config,
		settlement_config: mcr_settlement_config::Config,
	) -> Self {
		let commitment_events = match commitment_events {
			Some(stream) => Either::Left(stream),
			None => Either::Right(stream::pending()),
		};
		Task {
			executor,
			settlement_manager,
			da_db,
			commitment_events,
			execution_extension,
			settlement_config,
		}
	}

	fn settlement_enabled(&self) -> bool {
		matches!(&self.commitment_events, Either::Left(_))
	}
}

impl<E, S> Task<E, S>
where
	E: DynOptFinExecutor,
	S: McrSettlementManagerOperations,
{
	pub async fn run(
		mut self,
		da_connection_url: Url,
		stream_heartbeat_interval_sec: u64,
		allow_sync_from_zero: bool,
	) -> anyhow::Result<()> {
		let synced_height = self.da_db.get_synced_height().await?;
		// Sync Da from 0 is rejected by default. Only if forced it's allowed.
		if !allow_sync_from_zero && synced_height == 0 {
			return Err(anyhow::anyhow!("Da Sync from height zero is not allowed."));
		}

		info!("DA synced height: {:?}", synced_height);
		let mut da_client =
			GrpcDaSequencerClient::try_connect(&da_connection_url, stream_heartbeat_interval_sec)
				.await?;
		// TODO manage alert_channel in the issue #1169
		let (mut blocks_from_da, alert_channel) = da_client
			.stream_read_from_height(StreamReadFromHeightRequest { height: synced_height })
			.await
			.map_err(|e| {
				error!("Failed to stream blocks from DA: {:?}", e);
				e
			})?;

		loop {
			select! {
				Some(res) = blocks_from_da.next() => {
					let response = res.context("failed to get next block from DA")?;
					info!("Received block from DA");
					self.process_block_from_da(response).await?;
				}
				Some(res) = self.commitment_events.next() => {
					let event = res.context("failed to get commitment event")?;
					info!("Received commitment event");
					self.process_commitment_event(event).await?;
				}
				else => break,
			}
		}
		Ok(())
	}

	async fn process_block_from_da(&mut self, da_block: BlockV1) -> anyhow::Result<()> {
		let block_timestamp = chrono::Utc::now().timestamp_micros() as u64;

		info!(
			block_id = %hex::encode(da_block.block_id.clone()),
			da_height = da_block.height,
			time = block_timestamp,
			"Processing block from DA"
		);

		// check if the block has already been executed
		if self.da_db.has_executed_block(da_block.block_id.clone()).await? {
			info!("Block already executed: {:#?}. It will be skipped", da_block.block_id);
			return Ok(());
		}

		info!("process_block_from_da block not executed");

		// the da height must be greater than 1
		if da_block.height < 1 {
			anyhow::bail!("Invalid DA height: {:?}", da_block.height);
		}

		let block: Block = bcs::from_bytes(&da_block.data[..])?;

		// get the transactions
		let transactions_count = block.transactions().len();

		info!("process_block_from_da before execute");

		let span = info_span!(target: "movement_timing", "execute_block", block_id = %block.id());
		let commitment =
			self.execute_block_with_retries(block, block_timestamp).instrument(span).await?;

		info!("process_block_from_da block executed.");

		// decrement the number of transactions in flight on the executor
		self.executor.decrement_transactions_in_flight(transactions_count as u64);

		// mark the da_height - 1 as synced
		// we can't mark this height as synced because we must allow for the possibility of multiple blocks at the same height according to the m1 da specifications (which currently is built on celestia which itself allows more than one block at the same height)
		self.da_db.set_synced_height(da_block.height - 1).await?;

		// set the block as executed
		self.da_db.add_executed_block(da_block.block_id.clone()).await?;

		info!("process_block_from_da da db updated.");

		if self.settlement_enabled()
			// only settle every super_block_size_heights 
			&& da_block.height % self.settlement_config.settle.settlement_super_block_size == 0
		{
			info!("Posting block commitment via settlement manager");
			match &self.settlement_manager {
				Some(settlement_manager) => {
					match settlement_manager.post_block_commitment(commitment).await {
						Ok(_) => {}
						Err(e) => {
							error!("Failed to post block commitment: {:?}", e);
						}
					}
				}
				None => {
					error!("Settlement manager not initialized");
				}
			}
		} else {
			info!(block_id = ?da_block.block_id, "Skipping settlement");
		}

		Ok(())
	}
}

impl<E, S> Task<E, S>
where
	E: DynOptFinExecutor,
{
	/// Retries executing a block several times.
	/// This can be valid behavior if the block timestamps are too tightly clustered for the full node execution.
	/// However, this has to be deterministic, otherwise nodes will not be able to agree on the block commitment.
	async fn execute_block_with_retries(
		&mut self,
		block: Block,
		mut block_timestamp: u64,
	) -> anyhow::Result<BlockCommitment> {
		for _ in 0..self.execution_extension.block_retry_count {
			// we have to clone here because the block is supposed to be consumed by the executor
			match self.execute_block(block.clone(), block_timestamp).await {
				Ok(commitment) => return Ok(commitment),
				Err(e) => {
					info!("Failed to execute block: {:?}. Retrying", e);
					block_timestamp += self.execution_extension.block_retry_increment_microseconds;
					// increase the timestamp by 5 ms (5000 microseconds)
				}
			}
		}

		anyhow::bail!("Failed to execute block after 5 retries")
	}

	async fn execute_block(
		&mut self,
		block: Block,
		block_timestamp: u64,
	) -> anyhow::Result<BlockCommitment> {
		info!("settle execute_block start.");
		let block_id = block.id();
		let block_hash = HashValue::from_slice(block_id)?;

		// get the transactions
		let mut block_transactions = Vec::new();
		let block_metadata = self.executor.build_block_metadata(
			HashValue::sha3_256_of(block_id.as_bytes().as_slice()),
			block_timestamp,
		)?;

		info!("settle execute_block build_block_metadata done.");

		let block_metadata_transaction =
			SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(block_metadata));
		block_transactions.push(block_metadata_transaction);

		for transaction in block.transactions() {
			let signed_transaction: SignedTransaction = bcs::from_bytes(transaction.data())?;

			// check if the transaction has already been executed to prevent replays
			if self
				.executor
				.has_executed_transaction_opt(signed_transaction.committed_hash())?
			{
				continue;
			}

			info!(
				target: "movement_timing",
				tx_hash = %signed_transaction.committed_hash(),
				sender = %signed_transaction.sender(),
				sequence_number = signed_transaction.sequence_number(),
				"execute_transaction",
			);

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

	async fn process_commitment_event(
		&mut self,
		event: BlockCommitmentEvent,
	) -> anyhow::Result<()> {
		match event {
			BlockCommitmentEvent::Accepted(commitment) => {
				debug!("Commitment accepted: {:?}", commitment);
				self.executor
					.set_finalized_block_height(commitment.height())
					.context("failed to set finalized block height")
			}
			BlockCommitmentEvent::Rejected { height, reason } => {
				debug!("Commitment rejected: {:?} {:?}", height, reason);
				let current_head_height = self.executor.get_block_head_height()?;
				if height > current_head_height {
					// Nothing to revert
					Ok(())
				} else if self.settlement_config.settle.settlement_admin_mode {
					// Settlement admin assumes it's right.
					// It does not try to correct settled value on the L1.
					// Nor does it try to recompute its ledger.
					Ok(())
				} else {
					self.executor
						.revert_block_head_to(height - 1)
						.await
						.context(format!("failed to revert to block height {}", height - 1))
				}
			}
		}
	}
}
