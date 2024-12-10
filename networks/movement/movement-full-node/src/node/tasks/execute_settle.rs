//! Task module to execute blocks from the DA and process settlement.

use crate::node::da_db::DaDB;

use maptos_dof_execution::{
	DynOptFinExecutor, ExecutableBlock, ExecutableTransactions, HashValue,
	SignatureVerifiedTransaction, SignedTransaction, Transaction,
};
use mcr_settlement_manager::{CommitmentEventStream, McrSettlementManagerOperations};
use movement_da_light_node_client::MovementDaLightNodeClient;
use movement_da_light_node_proto::{
	blob_response, StreamReadFromHeightRequest, StreamReadFromHeightResponse,
};
use movement_types::block::{Block, BlockCommitment, BlockCommitmentEvent};

use anyhow::Context;
use futures::{future::Either, stream};
use movement_config::execution_extension;
use tokio::select;
use tokio_stream::{Stream, StreamExt};
use tracing::{debug, error, info, info_span, Instrument};

pub struct Task<E, S> {
	executor: E,
	settlement_manager: Option<S>,
	da_db: DaDB,
	da_light_node_client: MovementDaLightNodeClient,
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
		da_light_node_client: MovementDaLightNodeClient,
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
			da_light_node_client,
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
	pub async fn run(mut self) -> anyhow::Result<()> {
		let synced_height = self.da_db.get_synced_height().await?;
		info!("Synced height: {:?}", synced_height);
		let mut blocks_from_da = self
			.da_light_node_client
			.stream_read_from_height(StreamReadFromHeightRequest { height: synced_height })
			.await?;

		loop {
			select! {
				Some(res) = blocks_from_da.next() => {
					let response = res.context("failed to get next block from DA")?;
					self.process_block_from_da(response).await?;
				}
				Some(res) = self.commitment_events.next() => {
					let event = res.context("failed to get commitment event")?;
					self.process_commitment_event(event).await?;
				}
				else => break,
			}
		}
		Ok(())
	}

	async fn process_block_from_da(
		&mut self,
		response: StreamReadFromHeightResponse,
	) -> anyhow::Result<()> {
		// get the block
		let (block_bytes, block_timestamp, block_id, da_height) = match response
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

		info!(
			block_id = %hex::encode(block_id.clone()),
			da_height = da_height,
			time = block_timestamp,
			"Processing block from DA"
		);

		// check if the block has already been executed
		if self.da_db.has_executed_block(block_id.clone()).await? {
			info!("Block already executed: {:#?}. It will be skipped", block_id);
			return Ok(());
		}

		// the da height must be greater than 1
		if da_height < 2 {
			anyhow::bail!("Invalid DA height: {:?}", da_height);
		}

		let block: Block = bcs::from_bytes(&block_bytes[..])?;

		// get the transactions
		let transactions_count = block.transactions().len();
		let span = info_span!(target: "movement_timing", "execute_block", id = ?block_id);
		let commitment =
			self.execute_block_with_retries(block, block_timestamp).instrument(span).await?;

		// decrement the number of transactions in flight on the executor
		self.executor.decrement_transactions_in_flight(transactions_count as u64);

		// mark the da_height - 1 as synced
		// we can't mark this height as synced because we must allow for the possibility of multiple blocks at the same height according to the m1 da specifications (which currently is built on celestia which itself allows more than one block at the same height)
		self.da_db.set_synced_height(da_height - 1).await?;

		// set the block as executed
		self.da_db.add_executed_block(block_id.clone()).await?;

		if self.settlement_enabled()
			// only settle every super_block_size_heights 
			// todo: replace with timeslot tolerance
			&& da_height % self.settlement_config.settle.settlement_super_block_size == 0
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
			info!(block_id = ?block_id, "Skipping settlement");
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
					block_timestamp += self.execution_extension.block_retry_increment_microseconds; // increase the timestamp by 5 ms (5000 microseconds)
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
		let block_id = block.id();
		let block_hash = HashValue::from_slice(block.id())?;

		// get the transactions
		let mut block_transactions = Vec::new();
		let block_metadata = self.executor.build_block_metadata(
			HashValue::sha3_256_of(block_id.as_bytes().as_slice()),
			block_timestamp,
		)?;
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
