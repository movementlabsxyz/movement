//! Task processing incoming transactions for the opt API.

use super::Error;

use maptos_execution_util::config::mempool::Config as MempoolConfig;

use aptos_config::config::NodeConfig;
use aptos_mempool::core_mempool::CoreMempool;
use aptos_mempool::SubmissionStatus;
use aptos_mempool::{core_mempool::TimelineState, MempoolClientRequest};
use aptos_storage_interface::{state_view::LatestDbStateCheckpointView as _, DbReader};
use aptos_types::account_address::AccountAddress;
use aptos_types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::SignedTransaction;
use aptos_types::vm_status::DiscardedVMStatus;
use aptos_vm_validator::vm_validator::{self, TransactionValidation, VMValidator};
use std::collections::HashSet;

use crate::gc_account_sequence_number::UsedSequenceNumberPool;
use futures::channel::mpsc as futures_mpsc;
use futures::StreamExt;
use std::sync::{atomic::AtomicU64, Arc};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, info_span, warn, Instrument};

const GC_INTERVAL: Duration = Duration::from_secs(30);
const TOO_NEW_TOLERANCE: u64 = 32;

pub struct TransactionPipe {
	// The receiver for the mempool client.
	mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
	// Sender for the channel with accepted transactions.
	transaction_sender: mpsc::Sender<SignedTransaction>,
	// Access to the ledger DB. TODO: reuse an instance of VMValidator
	db_reader: Arc<dyn DbReader>,
	// State of the Aptos mempool
	core_mempool: CoreMempool,
	// Shared reference on the counter of transactions in flight.
	transactions_in_flight: Arc<AtomicU64>,
	// The configured limit on transactions in flight
	in_flight_limit: u64,
	// Timestamp of the last garbage collection
	last_gc: Instant,
	// The pool of used sequence numbers
	used_sequence_number_pool: UsedSequenceNumberPool,
	/// The accounts whitelisted for ingress
	whitelisted_accounts: Option<HashSet<AccountAddress>>,
}

enum SequenceNumberValidity {
	Valid(u64),
	Invalid(SubmissionStatus),
}

impl TransactionPipe {
	pub(crate) fn new(
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		transaction_sender: mpsc::Sender<SignedTransaction>,
		db_reader: Arc<dyn DbReader>,
		node_config: &NodeConfig,
		mempool_config: &MempoolConfig,
		transactions_in_flight: Arc<AtomicU64>,
		transactions_in_flight_limit: u64,
	) -> Result<Self, anyhow::Error> {
		Ok(TransactionPipe {
			mempool_client_receiver,
			transaction_sender,
			db_reader,
			core_mempool: CoreMempool::new(node_config),
			transactions_in_flight,
			in_flight_limit: transactions_in_flight_limit,
			last_gc: Instant::now(),
			used_sequence_number_pool: UsedSequenceNumberPool::new(
				mempool_config.sequence_number_ttl_ms,
				mempool_config.gc_slot_duration_ms,
			),
			whitelisted_accounts: mempool_config.whitelisted_accounts()?,
		})
	}

	pub fn is_whitelisted(&self, address: &AccountAddress) -> Result<bool, Error> {
		match &self.whitelisted_accounts {
			Some(whitelisted_accounts) => Ok(whitelisted_accounts.contains(address)),
			None => Ok(true),
		}
	}

	pub async fn run(mut self) -> Result<(), Error> {
		loop {
			self.tick().await?;
		}
	}

	/// Pipes a batch of transactions from the mempool to the transaction channel.
	/// todo: it may be wise to move the batching logic up a level to the consuming structs.
	pub(crate) async fn tick(&mut self) -> Result<(), Error> {
		let next = self.mempool_client_receiver.next().await;
		if let Some(request) = next {
			match request {
				MempoolClientRequest::SubmitTransaction(transaction, callback) => {
					let span = info_span!(
						target: "movement_timing",
						"submit_transaction",
						tx_hash = %transaction.committed_hash(),
						sender = %transaction.sender(),
						sequence_number = transaction.sequence_number(),
					);
					let status = self.submit_transaction(transaction).instrument(span).await?;
					callback.send(Ok(status)).unwrap_or_else(|_| {
						debug!("SubmitTransaction request canceled");
					});
				}
				MempoolClientRequest::GetTransactionByHash(hash, sender) => {
					let mempool_result = self.core_mempool.get_by_hash(hash);
					sender.send(mempool_result).unwrap_or_else(|_| {
						debug!("GetTransactionByHash request canceled");
					});
				}
			}
		} else {
			return Err(Error::InputClosed);
		}

		if self.last_gc.elapsed() >= GC_INTERVAL {
			// todo: these will be slightly off, but gc does not need to be exact
			let now = Instant::now();
			let epoch_ms_now = chrono::Utc::now().timestamp_millis() as u64;
			self.used_sequence_number_pool.gc(epoch_ms_now);
			self.core_mempool.gc();
			self.last_gc = now;
		}

		Ok(())
	}

	fn has_invalid_sequence_number(
		&self,
		transaction: &SignedTransaction,
	) -> Result<SequenceNumberValidity, Error> {
		// check against the used sequence number pool
		let used_sequence_number = self
			.used_sequence_number_pool
			.get_sequence_number(&transaction.sender())
			.unwrap_or(0);

		// validate against the state view
		let state_view = self.db_reader.latest_state_checkpoint_view().map_err(|e| {
			Error::InternalError(format!("Failed to get latest state view: {:?}", e))
		})?;

		// this checks that the sequence number is too old or too new
		let committed_sequence_number =
			vm_validator::get_account_sequence_number(&state_view, transaction.sender())?;

		debug!(
			"Used sequence number: {:?} Committed sequence number: {:?}",
			used_sequence_number, committed_sequence_number
		);
		let min_used_sequence_number =
			if used_sequence_number > 0 { used_sequence_number + 1 } else { 0 };

		let min_sequence_number = (min_used_sequence_number).max(committed_sequence_number);

		let max_sequence_number = committed_sequence_number + TOO_NEW_TOLERANCE;

		info!(
			"min_sequence_number: {:?} max_sequence_number: {:?} transaction_sequence_number {:?}",
			min_sequence_number,
			max_sequence_number,
			transaction.sequence_number()
		);

		if transaction.sequence_number() < min_sequence_number {
			info!("Transaction sequence number too old: {:?}", transaction.sequence_number());
			return Ok(SequenceNumberValidity::Invalid((
				MempoolStatus::new(MempoolStatusCode::InvalidSeqNumber),
				Some(DiscardedVMStatus::SEQUENCE_NUMBER_TOO_OLD),
			)));
		}

		if transaction.sequence_number() > max_sequence_number {
			info!("Transaction sequence number too new: {:?}", transaction.sequence_number());
			return Ok(SequenceNumberValidity::Invalid((
				MempoolStatus::new(MempoolStatusCode::InvalidSeqNumber),
				Some(DiscardedVMStatus::SEQUENCE_NUMBER_TOO_NEW),
			)));
		}

		Ok(SequenceNumberValidity::Valid(committed_sequence_number))
	}

	async fn submit_transaction(
		&mut self,
		transaction: SignedTransaction,
	) -> Result<SubmissionStatus, Error> {
		// Check whether the account is whitelisted
		if !self.is_whitelisted(&transaction.sender())? {
			return Ok((MempoolStatus::new(MempoolStatusCode::TooManyTransactions), None));
		}

		// For now, we are going to consider a transaction in flight until it exits the mempool and is sent to the DA as is indicated by WriteBatch.
		let in_flight = self.transactions_in_flight.load(std::sync::atomic::Ordering::Relaxed);
		info!(
			target: "movement_timing",
			in_flight = %in_flight,
			"transactions_in_flight"
		);
		if in_flight > self.in_flight_limit {
			info!(
				target: "movement_timing",
				"shedding_load"
			);
			let status = MempoolStatus::new(MempoolStatusCode::MempoolIsFull);
			return Ok((status, None));
		}

		// Pre-execute Tx to validate its content.
		// Re-create the validator for each Tx because it uses a frozen version of the ledger.
		let vm_validator = VMValidator::new(Arc::clone(&self.db_reader));
		let tx_result = vm_validator.validate_transaction(transaction.clone())?;
		match tx_result.status() {
			Some(_) => {
				let ms = MempoolStatus::new(MempoolStatusCode::VmError);
				debug!("Transaction not accepted: {:?}", tx_result.status());
				return Ok((ms, tx_result.status()));
			}
			None => {
				debug!("Transaction accepted by VM: {:?}", transaction);
			}
		}

		let sequence_number = match self.has_invalid_sequence_number(&transaction)? {
			SequenceNumberValidity::Valid(sequence_number) => sequence_number,
			SequenceNumberValidity::Invalid(status) => {
				return Ok(status);
			}
		};

		// Add the txn for future validation
		debug!("Adding transaction to mempool: {:?} {:?}", transaction, sequence_number);
		let status = self.core_mempool.add_txn(
			transaction.clone(),
			0,
			sequence_number,
			TimelineState::NonQualified,
			true,
		);

		match status.code {
			MempoolStatusCode::Accepted => {
				debug!("Transaction accepted: {:?}", transaction);
				let sender = transaction.sender();
				let transaction_sequence_number = transaction.sequence_number();
				self.transaction_sender
					.send(transaction)
					.await
					.map_err(|e| anyhow::anyhow!("Error sending transaction: {:?}", e))?;
				// increment transactions in flight
				self.transactions_in_flight.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
				self.core_mempool.commit_transaction(&sender, sequence_number);

				// update the used sequence number pool
				info!(
					"Setting used sequence number for {:?} to {:?}",
					sender, transaction_sequence_number
				);
				self.used_sequence_number_pool.set_sequence_number(
					&sender,
					transaction_sequence_number,
					chrono::Utc::now().timestamp_millis() as u64,
				);
			}
			_ => {
				warn!("Transaction not accepted: {:?}", status);
			}
		}

		// report status
		Ok((status, None))
	}
}

#[cfg(test)]
mod tests {

	use std::collections::BTreeSet;

	use super::*;
	use crate::{Context, Executor, Service};
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
	use aptos_crypto::HashValue;
	use aptos_types::{
		account_config,
		block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
		block_metadata::BlockMetadata,
		test_helpers::transaction_test_helpers,
		transaction::{
			signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
			Transaction,
		},
	};
	use aptos_vm_genesis::GENESIS_KEYPAIR;
	use futures::channel::oneshot;
	use futures::SinkExt;
	use maptos_execution_util::config::chain::Config;
	use tempfile::TempDir;

	fn setup() -> (Context, TransactionPipe, mpsc::Receiver<SignedTransaction>, TempDir) {
		let (tx_sender, tx_receiver) = mpsc::channel(16);
		let (executor, tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone()).unwrap();
		let (context, background) = executor.background(tx_sender).unwrap();
		let transaction_pipe = background.into_transaction_pipe();
		(context, transaction_pipe, tx_receiver, tempdir)
	}

	fn create_signed_transaction(sequence_number: u64, chain_config: &Config) -> SignedTransaction {
		let address = account_config::aptos_test_root_address();
		transaction_test_helpers::get_test_txn_with_chain_id(
			address,
			sequence_number,
			&GENESIS_KEYPAIR.0,
			GENESIS_KEYPAIR.1.clone(),
			chain_config.maptos_chain_id.clone(), // This is the value used in aptos testing code.
		)
	}

	#[tokio::test]
	async fn test_pipe_mempool() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (context, mut transaction_pipe, mut tx_receiver, _tempdir) = setup();
		let user_transaction = create_signed_transaction(1, &maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		context
			.mempool_client_sender()
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		transaction_pipe.tick().await?;

		// receive the callback
		let (status, _vm_status_code) = callback.await??;
		assert_eq!(status.code, MempoolStatusCode::Accepted);

		// receive the transaction
		let received_transaction = tx_receiver.recv().await.unwrap();
		assert_eq!(received_transaction, user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_cancellation() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (context, mut transaction_pipe, _tx_receiver, _tempdir) = setup();
		let user_transaction = create_signed_transaction(1, &maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		context
			.mempool_client_sender()
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// drop the callback to simulate cancellation of the request
		drop(callback);

		// tick the transaction pipe, should succeed
		transaction_pipe.tick().await?;

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_with_duplicate_transaction() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (context, mut transaction_pipe, mut tx_receiver, _tempdir) = setup();
		let mut mempool_client_sender = context.mempool_client_sender();
		let user_transaction = create_signed_transaction(1, &maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		transaction_pipe.tick().await?;

		// receive the callback
		let (status, _vm_status_code) = callback.await??;
		assert_eq!(status.code, MempoolStatusCode::Accepted);

		// receive the transaction
		let received_transaction =
			tx_receiver.recv().await.ok_or(anyhow::anyhow!("No transaction received"))?;
		assert_eq!(received_transaction, user_transaction);

		// send the same transaction again
		let (req_sender, callback) = oneshot::channel();
		mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		transaction_pipe.tick().await?;

		callback.await??;

		// assert that there is no new transaction
		assert!(tx_receiver.try_recv().is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let (context, mut transaction_pipe, mut tx_receiver, _tempdir) = setup();
		let service = Service::new(&context);

		#[allow(unreachable_code)]
		let mempool_handle = tokio::spawn(async move {
			loop {
				transaction_pipe.tick().await?;
			}
			Ok(()) as Result<(), anyhow::Error>
		});

		let api = service.get_apis();
		let user_transaction = create_signed_transaction(1, &context.config().chain);
		let comparison_user_transaction = user_transaction.clone();
		let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;
		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;
		let received_transaction = tx_receiver.recv().await.unwrap();
		assert_eq!(received_transaction, comparison_user_transaction);

		mempool_handle.abort();

		Ok(())
	}

	#[tokio::test]
	async fn test_repeated_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let (context, mut transaction_pipe, mut tx_receiver, _tempdir) = setup();
		let service = Service::new(&context);

		#[allow(unreachable_code)]
		let mempool_handle = tokio::spawn(async move {
			loop {
				transaction_pipe.tick().await?;
			}
			Ok(()) as Result<(), anyhow::Error>
		});

		let api = service.get_apis();
		let mut user_transactions = BTreeSet::new();
		let mut comparison_user_transactions = BTreeSet::new();
		for i in 1..25 {
			let user_transaction = create_signed_transaction(i, &context.config().chain);
			let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;
			user_transactions.insert(bcs_user_transaction.clone());

			let request =
				SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
			api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

			let received_transaction = tx_receiver.recv().await.unwrap();
			let bcs_received_transaction = bcs::to_bytes(&received_transaction)?;
			comparison_user_transactions.insert(bcs_received_transaction.clone());
		}

		assert_eq!(user_transactions.len(), comparison_user_transactions.len());
		assert_eq!(user_transactions, comparison_user_transactions);

		mempool_handle.abort();

		Ok(())
	}

	#[tokio::test]
	async fn test_cannot_submit_too_new() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (_context, mut transaction_pipe, _tx_receiver, _tempdir) = setup();

		// submit a transaction with a valid sequence number
		let user_transaction = create_signed_transaction(0, &maptos_config);
		let (mempool_status, _) = transaction_pipe.submit_transaction(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit a transaction with a sequence number that is too new
		let user_transaction = create_signed_transaction(34, &maptos_config);
		let (mempool_status, _) = transaction_pipe.submit_transaction(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::InvalidSeqNumber);

		// submit one signed transaction with a sequence number that is too new for the vm but not for the mempool
		let user_transaction = create_signed_transaction(5, &maptos_config);
		let (mempool_status, _) = transaction_pipe.submit_transaction(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit a transaction with the same sequence number as the previous one
		let user_transaction = create_signed_transaction(5, &maptos_config);
		let (mempool_status, _) = transaction_pipe.submit_transaction(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::InvalidSeqNumber);

		Ok(())
	}

	#[tokio::test]
	async fn test_sequence_number_too_old() -> Result<(), anyhow::Error> {
		let (tx_sender, _tx_receiver) = mpsc::channel(16);
		let (executor, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let (context, background) = executor.background(tx_sender)?;
		let mut transaction_pipe = background.into_transaction_pipe();

		#[allow(unreachable_code)]
		let mempool_handle = tokio::spawn(async move {
			loop {
				transaction_pipe.tick().await?;
			}
			Ok(()) as Result<(), anyhow::Error>
		});

		let tx = create_signed_transaction(0, &context.config().chain);

		// Commit the first transaction to a block
		let block_id = HashValue::random();
		let block_metadata = Transaction::BlockMetadata(BlockMetadata::new(
			block_id,
			0,
			0,
			executor.signer.author(),
			vec![],
			vec![],
			chrono::Utc::now().timestamp_micros() as u64,
		));
		let txs = ExecutableTransactions::Unsharded(
			[block_metadata, Transaction::UserTransaction(tx)]
				.into_iter()
				.map(SignatureVerifiedTransaction::Valid)
				.collect(),
		);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(block).await?;

		{
			let state_view = executor
				.db_reader()
				.latest_state_checkpoint_view()
				.expect("Failed to get latest state checkpoint view.");
			let account_address = account_config::aptos_test_root_address();
			let sequence_number =
				vm_validator::get_account_sequence_number(&state_view, account_address)?;
			assert_eq!(sequence_number, 1);
		}

		// Create another transaction using the already used sequence number
		let tx = create_signed_transaction(0, &context.config().chain);

		// send the transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		context
			.mempool_client_sender()
			.send(MempoolClientRequest::SubmitTransaction(tx, req_sender))
			.await?;

		let status = callback.await??;

		assert_eq!(status.0.code, MempoolStatusCode::VmError);
		let vm_status = status.1.unwrap();
		assert_eq!(vm_status, DiscardedVMStatus::SEQUENCE_NUMBER_TOO_OLD);

		mempool_handle.abort();

		Ok(())
	}
}
