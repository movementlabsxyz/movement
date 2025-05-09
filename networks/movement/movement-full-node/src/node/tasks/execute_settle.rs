//! Task module to execute blocks from the DA and process settlement.
use crate::node::da_db::DaDB;
use crate::node::tasks::state_verifier::StateVerifier;
use anyhow::Context;
use futures::{future::Either, stream};
use maptos_dof_execution::{
	DynOptFinExecutor, ExecutableBlock, ExecutableTransactions, HashValue,
	SignatureVerifiedTransaction, SignedTransaction, Transaction,
};
use maptos_opt_executor::executor::ExecutionState;
use mcr_settlement_manager::{CommitmentEventStream, McrSettlementManagerOperations};
use movement_config::execution_extension;
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_client::GrpcDaSequencerClient;
use movement_da_sequencer_proto::BlockV1;
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer_loader::identifiers::SignerIdentifier;
use movement_signer_loader::{Load, LoadedSigner};
use movement_types::block::{Block, BlockCommitment, BlockCommitmentEvent};
use tokio::select;
use tokio_stream::{Stream, StreamExt};
use tracing::{debug, error, info, info_span, Instrument};
use url::Url;

pub struct Task<E, S> {
	executor: Option<E>,
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
			executor: Some(executor),
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
	E: DynOptFinExecutor + std::marker::Send + 'static,
	S: McrSettlementManagerOperations + std::marker::Send,
{
	pub async fn run(
		mut self,
		da_connection_url: Url,
		stream_heartbeat_interval_sec: u64,
		allow_sync_from_zero: bool,
		propagate_execution_state: bool,
		da_batch_signer: &SignerIdentifier,
	) -> anyhow::Result<()> {
		let synced_height = self.da_db.get_synced_height()?;
		// Sync Da from 0 is rejected by default. Only if forced it's allowed.
		if !allow_sync_from_zero && synced_height == 0 {
			return Err(anyhow::anyhow!("Da Sync from height zero is not allowed."));
		}

		let mut node_state_verifier = StateVerifier::new();

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
					let span = info_span!(target: "movement_timing", "process_block_from_da", block_id = %hex::encode(response.block_id.clone()));
					tracing::info!("Receive state from DA: {:?}",response.node_state);
					if let Some(main_state) = response.node_state {
						node_state_verifier.add_state(main_state);
					}
					let new_state = self.process_block_from_da(response).instrument(span).await?;
					tracing::info!("New state after execution: {new_state:?}");
					if let Some(new_state) = new_state {
						if !node_state_verifier.validate(&new_state) {
							let main_node_state = node_state_verifier.get_state(new_state.block_height.into());
							tracing::error!("State from Da verification failed, local node state: {new_state:?} main_node_state:{main_node_state:?}");
							break;
						}

						// If main node send new execution result state
						if propagate_execution_state {
							tokio::spawn({
								let mut client = da_client.clone();
								let signer: LoadedSigner<Ed25519> = da_batch_signer.load().await?;
								let state = movement_da_sequencer_proto::MainNodeState {
									block_height: new_state.block_height,
									ledger_timestamp:  new_state.ledger_timestamp,
									ledger_version: new_state.ledger_version,

								};
								async move {
									if let Err(err) = client.send_state(&signer, state).await {
										tracing::error!("Send execution state to da sequencer failed : {err}");
									}
								}
							});

						}
					}
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

	async fn process_block_from_da(
		&mut self,
		da_block: BlockV1,
	) -> anyhow::Result<Option<ExecutionState>> {
		let da_block_height = da_block.height;
		let block_id = da_block.block_id.clone();

		//only on execution at a time
		// Break if we try to do several.
		if self.executor.is_none() {
			anyhow::bail!(
				"Block execution failed, executor already in use. Block {:?} failed.",
				block_id
			);
		}

		let (exec_result, executor) = tokio::task::spawn_blocking({
			let da_db = self.da_db.clone();
			let block_retry_count = self.execution_extension.block_retry_count;
			let block_retry_increment_microseconds =
				self.execution_extension.block_retry_increment_microseconds;
			let mut executor = self.executor.take().unwrap(); // unwrap tested just before.
			move || {
				// check if the block has already been executed
				if da_db.has_executed_block(da_block.block_id.clone())? {
					info!("Block already executed: {:#?}. It will be skipped", da_block.block_id);
					return Ok((None, executor));
				}

				// the da height must be greater than 1
				if da_block.height < 1 {
					anyhow::bail!("Invalid DA height: {:?}", da_block.height);
				}

				let block: Block = bcs::from_bytes(&da_block.data[..])?;

				info!(
					block_id = %hex::encode(&block.id()),
					da_height = da_block_height,
					time = block.timestamp(),
					"Processing block from DA"
				);

				// get the transactions
				let transactions_count = block.transactions().len();

				let block_timestamp = block.timestamp();

				let exec_result = Self::execute_block_with_retries(
					&mut executor,
					block,
					block_timestamp,
					block_retry_count,
					block_retry_increment_microseconds,
				)?;

				// decrement the number of transactions in flight on the executor
				executor.decrement_transactions_in_flight(transactions_count as u64);

				da_db.set_synced_height(da_block.height)?;

				// set the block as executed
				da_db.add_executed_block(da_block.block_id.clone())?;

				Ok((Some(exec_result), executor))
			}
		})
		.await??;
		self.executor.replace(executor);

		let (commitment, new_ledger_state) = match exec_result {
			Some(exec_result) => exec_result,
			None => return Ok(None),
		};

		if self.settlement_enabled()
			// only settle every super_block_size_heights 
			&& da_block_height % self.settlement_config.settle.settlement_super_block_size == 0
		{
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

		Ok(Some(new_ledger_state))
	}
}

impl<E, S> Task<E, S>
where
	E: DynOptFinExecutor,
{
	/// Retries executing a block several times.
	/// This can be valid behavior if the block timestamps are too tightly clustered for the full node execution.
	/// However, this has to be deterministic, otherwise nodes will not be able to agree on the block commitment.
	fn execute_block_with_retries(
		executor: &mut E,
		block: Block,
		mut block_timestamp: u64,
		block_retry_count: u64,
		block_retry_increment_microseconds: u64,
	) -> anyhow::Result<(BlockCommitment, ExecutionState)> {
		for _ in 0..block_retry_count {
			// we have to clone here because the block is supposed to be consumed by the executor
			match Self::execute_block(executor, block.clone(), block_timestamp) {
				Ok(commitment) => return Ok(commitment),
				Err(e) => {
					info!("Failed to execute block: {:?}. Retrying", e);
					block_timestamp += block_retry_increment_microseconds;
					// increase the timestamp by 5 ms (5000 microseconds)
				}
			}
		}

		anyhow::bail!("Failed to execute block after 5 retries")
	}

	fn execute_block(
		executor: &mut E,
		block: Block,
		block_timestamp: u64,
	) -> anyhow::Result<(BlockCommitment, ExecutionState)> {
		let block_id = block.id();

		let _span = info_span!("execute_block", %block_id).entered();

		let block_hash = HashValue::from_slice(block_id)?;

		// get the transactions
		let mut block_transactions = Vec::new();
		let block_metadata = executor.build_block_metadata(
			HashValue::sha3_256_of(block_id.as_bytes().as_slice()),
			block_timestamp,
		)?;

		let block_metadata_transaction =
			SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(block_metadata));
		block_transactions.push(block_metadata_transaction);

		for transaction in block.transactions() {
			let signed_transaction: SignedTransaction = bcs::from_bytes(transaction.data())?;

			// check if the transaction has already been executed to prevent replays
			if executor.has_executed_transaction_opt(signed_transaction.committed_hash())? {
				continue;
			}

			debug!(
				target: "movement_timing",
				tx_hash = %signed_transaction.committed_hash(),
				sender = %signed_transaction.sender(),
				sequence_number = signed_transaction.sequence_number(),
				timestamp_secs = signed_transaction.expiration_timestamp_secs(),
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
		let exec_result = executor.execute_block_opt(executable_block)?;

		debug!("Executed block: {}", block_id);

		Ok(exec_result)
	}

	async fn process_commitment_event(
		&mut self,
		event: BlockCommitmentEvent,
	) -> anyhow::Result<()> {
		let executor = match self.executor {
			Some(ref executor) => executor,
			None => anyhow::bail!("Bock commitment failed, executor not present.",),
		};
		match event {
			BlockCommitmentEvent::Accepted(commitment) => {
				debug!("Commitment accepted: {:?}", commitment);
				executor
					.set_finalized_block_height(commitment.height())
					.context("failed to set finalized block height")
			}
			BlockCommitmentEvent::Rejected { height, reason } => {
				debug!("Commitment rejected: {:?} {:?}", height, reason);
				let current_head_height = executor.get_block_head_height()?;
				if height > current_head_height {
					// Nothing to revert
					Ok(())
				} else if self.settlement_config.settle.settlement_admin_mode {
					// Settlement admin assumes it's right.
					// It does not try to correct settled value on the L1.
					// Nor does it try to recompute its ledger.
					Ok(())
				} else {
					executor
						.revert_block_head_to(height - 1)
						.await
						.context(format!("failed to revert to block height {}", height - 1))
				}
			}
		}
	}
}
