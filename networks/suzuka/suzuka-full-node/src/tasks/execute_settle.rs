//! Task module to execute blocks from the DA and process settlement.

use crate::da_db::DaDB;

use m1_da_light_node_client::{
	blob_response, LightNodeServiceClient, StreamReadFromHeightRequest,
	StreamReadFromHeightResponse,
};
use maptos_dof_execution::{
	DynOptFinExecutor, ExecutableBlock, ExecutableTransactions, HashValue,
	SignatureVerifiedTransaction, SignedTransaction, Transaction,
};
use mcr_settlement_manager::{CommitmentEventStream, McrSettlementManagerOperations};
use movement_types::block::{Block, BlockCommitment, BlockCommitmentEvent};

use anyhow::Context;
use futures::{future::Either, stream};
use suzuka_config::execution_extension;
use tokio::select;
use tokio_stream::{Stream, StreamExt};
use tracing::{debug, error, info, info_span, Instrument};

pub struct Task<E, S> {
	executor: E,
	settlement_manager: S,
	da_db: DaDB,
	da_light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
	// Stream receiving commitment events, conditionally enabled
	commitment_events:
		Either<CommitmentEventStream, stream::Pending<<CommitmentEventStream as Stream>::Item>>,
	execution_extension: execution_extension::Config,
}

impl<E, S> Task<E, S> {
	pub(crate) fn new(
		executor: E,
		settlement_manager: S,
		da_db: DaDB,
		da_light_node_client: LightNodeServiceClient<tonic::transport::Channel>,
		commitment_events: Option<CommitmentEventStream>,
		execution_extension: execution_extension::Config,
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
		// TODO: this is a temporary solution to rollover the genesis block, really this
		// (a) needs to be read from the DA and
		// (b) requires modifications to Aptos Core.
		self.executor.rollover_genesis_block().await?;

		let synced_height = self.da_db.get_synced_height().await?;
		info!("Synced height: {:?}", synced_height);
		let mut blocks_from_da = self
			.da_light_node_client
			.stream_read_from_height(StreamReadFromHeightRequest { height: synced_height })
			.await?
			.into_inner();

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
			block_id = %block_id,
			da_height = da_height,
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

		// decompress the block bytes
		let block = tokio::task::spawn_blocking(move || {
			let decompressed_block_bytes = zstd::decode_all(&block_bytes[..])?;
			let block: Block = bcs::from_bytes(&decompressed_block_bytes)?;
			Ok::<Block, anyhow::Error>(block)
		})
		.await??;

		// get the transactions count before the block is consumed
		let transactions_count = block.transactions().len();
		let span = info_span!(target: "movement_telemetry", "execute_block", id = %block_id);
		let commitment =
			self.execute_block_with_retries(block, block_timestamp).instrument(span).await?;

		// decrement the number of transactions in flight on the executor
		self.executor.decrement_transactions_in_flight(transactions_count as u64);

		// mark the da_height - 1 as synced
		// we can't mark this height as synced because we must allow for the possibility of multiple blocks at the same height according to the m1 da specifications (which currently is built on celestia which itself allows more than one block at the same height)
		self.da_db.set_synced_height(da_height - 1).await?;

		// set the block as executed
		self.da_db.add_executed_block(block_id.clone()).await?;

		// todo: this needs defaults
		if self.settlement_enabled() {
			info!("Posting block commitment via settlement manager");
			match self.settlement_manager.post_block_commitment(commitment).await {
				Ok(_) => {}
				Err(e) => {
					error!("Failed to post block commitment: {:?}", e);
				}
			}
		} else {
			info!(block_id = %block_id, "Skipping settlement");
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
			let signed_transaction: SignedTransaction = serde_json::from_slice(transaction.data())?;

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
