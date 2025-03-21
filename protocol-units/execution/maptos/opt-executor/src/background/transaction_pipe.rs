//! Task processing incoming transactions for the opt API.

use super::Error;
use crate::executor::TxExecutionResult;
use crate::gc_account_sequence_number::UsedSequenceNumberPool;
use aptos_account_whitelist::config::Config as WhitelistConfig;
use aptos_config::config::NodeConfig;
use aptos_mempool::core_mempool::CoreMempool;
use aptos_mempool::SubmissionStatus;
use aptos_mempool::{core_mempool::TimelineState, MempoolClientRequest};
use aptos_storage_interface::state_view::LatestDbStateCheckpointView;
use aptos_storage_interface::DbReader;
use aptos_types::account_address::AccountAddress;
use aptos_types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::SignedTransaction;
use aptos_types::transaction::TransactionStatus;
use aptos_vm_validator::vm_validator::get_account_sequence_number;
use aptos_vm_validator::vm_validator::{TransactionValidation, VMValidator};
use futures::channel::mpsc as futures_mpsc;
use maptos_execution_util::config::mempool::Config as MempoolConfig;
use movement_collections::garbage::counted::GcCounter;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, info_span, warn, Instrument};

const GC_INTERVAL: Duration = Duration::from_secs(30);
const MEMPOOL_INTERVAL: Duration = Duration::from_millis(240); // this is based on slot times and global TCP RTT, essentially we expect to collect all transactions sent in the same slot in around 240ms

pub struct TransactionPipe {
	// The receiver for Tx execution to commit in the mempool.
	mempool_commit_tx_receiver: futures_mpsc::Receiver<Vec<TxExecutionResult>>,
	// The receiver for the mempool client.
	mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
	// Sender for the channel with accepted transactions.
	transaction_sender: mpsc::Sender<(u64, SignedTransaction)>,
	// Access to the ledger DB. TODO: reuse an instance of VMValidator
	db_reader: Arc<dyn DbReader>,
	// State of the Aptos mempool
	core_mempool: CoreMempool,
	// Shared reference on the counter of transactions in flight.
	transactions_in_flight: Arc<RwLock<GcCounter>>,
	// The configured limit on transactions in flight
	in_flight_limit: Option<u64>,
	// Timestamp of the last mempool send
	last_mempool_send: Instant,
	// Timestamp of the last garbage collection
	last_gc: Instant,
	// The pool of used sequence numbers
	used_sequence_number_pool: UsedSequenceNumberPool,
	/// The accounts whitelisted for ingress
	whitelisted_accounts: Option<HashSet<AccountAddress>>,
}

impl TransactionPipe {
	pub(crate) fn new(
		mempool_commit_tx_receiver: futures_mpsc::Receiver<Vec<TxExecutionResult>>, // Sender, seq number)
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		transaction_sender: mpsc::Sender<(u64, SignedTransaction)>,
		db_reader: Arc<dyn DbReader>,
		node_config: &NodeConfig,
		mempool_config: &MempoolConfig,
		whitelist_config: &WhitelistConfig,
		transactions_in_flight: Arc<RwLock<GcCounter>>,
		transactions_in_flight_limit: Option<u64>,
	) -> Result<Self, anyhow::Error> {
		let whitelisted_accounts = whitelist_config.whitelisted_accounts()?;
		info!("Whitelisted accounts: {:?}", whitelisted_accounts);

		Ok(TransactionPipe {
			mempool_commit_tx_receiver,
			mempool_client_receiver,
			transaction_sender,
			db_reader,
			core_mempool: CoreMempool::new(node_config),
			transactions_in_flight,
			in_flight_limit: transactions_in_flight_limit,
			last_mempool_send: Instant::now(),
			last_gc: Instant::now(),
			used_sequence_number_pool: UsedSequenceNumberPool::new(
				mempool_config.sequence_number_ttl_ms,
				mempool_config.gc_slot_duration_ms,
			),
			whitelisted_accounts,
		})
	}

	pub fn is_whitelisted(&self, address: &AccountAddress) -> Result<bool, Error> {
		match &self.whitelisted_accounts {
			Some(whitelisted_accounts) => {
				let whitelisted = whitelisted_accounts.contains(address);
				info!("Checking if account {:?} is whitelisted: {:?}", address, whitelisted);
				Ok(whitelisted)
			}
			None => Ok(true),
		}
	}

	pub async fn run(mut self) -> Result<(), Error> {
		loop {
			self.tick().await?;
			let _ = tokio::time::sleep(tokio::time::Duration::from_millis(100));
		}
	}

	pub async fn tick(&mut self) -> Result<(), Error> {
		self.tick_requests().await?;
		self.tick_mempool_sender().await?;
		self.tick_commit_tx().await?;
		self.tick_gc();
		Ok(())
	}

	/// Pipes a batch of transactions from the executor to commit then in the Aptos mempool.
	pub(crate) async fn tick_commit_tx(&mut self) -> Result<(), Error> {
		match self.mempool_commit_tx_receiver.try_next() {
			Ok(Some(batch)) => {
				for tx_result in batch {
					if let TransactionStatus::Discard(discard_status) = tx_result.status {
						tracing::info!(
							"Transaction pipe, mempool rejecting Tx:{} with status:{:?}",
							tx_result.hash,
							discard_status
						);
						self.core_mempool.reject_transaction(
							&tx_result.sender,
							tx_result.seq_number,
							&tx_result.hash,
							&discard_status,
						)
					} else {
						tracing::info!(
							tx_hash = %tx_result.hash,
							sender = %tx_result.sender,
							sequence_number = %tx_result.seq_number,
							"mempool rejected transaction",
						);
					}
				}
			}
			Ok(None) => return Err(Error::InputClosed),
			Err(_) => (),
		}
		Ok(())
	}

	/// Pipes a batch of transactions from the mempool to the transaction channel.
	/// todo: it may be wise to move the batching logic up a level to the consuming structs.
	pub(crate) async fn tick_requests(&mut self) -> Result<(), Error> {
		// try to immediately get the next request
		// if we don't do this, then the `tick_mempool_sender` process would be held up by receiving a new transaction

		// todo: for some reason this causes the core_mempool API to flag the receiver as gone. This workaround needs to be reinvestigated.
		// we use select to make this timeout after 1ms
		/*let timeout = tokio::time::sleep(Duration::from_millis(1));

		let next = tokio::select! {
			next = self.mempool_client_receiver.next() => next, // If received, process it
			_ = timeout => None, // If timeout, return None
		};*/

		// this also causes the core_mempool API to flag the receiver as gone
		// let next = self.mempool_client_receiver.try_next().map_err(|_| Error::InputClosed)?;
		match self.mempool_client_receiver.try_next() {
			Ok(Some(request)) => match request {
				MempoolClientRequest::SubmitTransaction(transaction, callback) => {
					let span = info_span!(
						target: "movement_timing",
						"submit_transaction",
						tx_hash = %transaction.committed_hash(),
						sender = %transaction.sender(),
						sequence_number = transaction.sequence_number(),
					);
					let status =
						self.add_transaction_to_aptos_mempool(transaction).instrument(span).await?;

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
			},
			Ok(None) => return Err(Error::InputClosed),
			Err(_) => (),
		}

		Ok(())
	}

	pub(crate) async fn tick_mempool_sender(&mut self) -> Result<(), Error> {
		if self.last_mempool_send.elapsed() > MEMPOOL_INTERVAL {
			// pop some transactions from the mempool
			let transactions = self.core_mempool.get_batch_with_ranking_score(
				1024 * 8,           // todo: move out to config
				1024 * 1024 * 1024, // todo: move out to config
				true,               // allows the mempool to return batch before one is full
				BTreeMap::new(),
			);
			debug!("Sending {:?} transactions to the transaction channel", transactions.len());

			// send them to the transaction channel
			for (transaction, ranking_score) in transactions {
				// clone the channel sender
				let sender = self.transaction_sender.clone();

				// grab the sender and sequence number
				let transaction_sender = transaction.sender();
				let sequence_number = transaction.sequence_number();

				// application priority for movement is the inverse of the ranking score
				let application_priority = u64::MAX - ranking_score;
				let _ = sender.send((application_priority, transaction)).await;

				// commit the transaction now that we have sent it
				debug!(
					target: "movement_timing",
					sender = %transaction_sender,
					sequence_number = sequence_number,
					"Tx sent to Tx ingress"
				);
				self.core_mempool.commit_transaction(&transaction_sender, sequence_number);
			}

			// update the last send
			self.last_mempool_send = Instant::now();
		}

		Ok(())
	}

	pub(crate) fn tick_gc(&mut self) {
		if self.last_gc.elapsed() >= GC_INTERVAL {
			// todo: these will be slightly off, but gc does not need to be exact
			let now = Instant::now();
			let epoch_ms_now = chrono::Utc::now().timestamp_millis() as u64;

			// garbage collect the used sequence number pool
			self.used_sequence_number_pool.gc(epoch_ms_now);

			// garbage collect the transactions in flight
			{
				// unwrap because failure indicates poisoned lock
				let mut transactions_in_flight = self.transactions_in_flight.write().unwrap();
				transactions_in_flight.gc(epoch_ms_now);
			}

			// garbage collect the core mempool
			self.core_mempool.gc();

			self.last_gc = now;
		}
	}

	// Adds a transaction to the mempool.
	async fn add_transaction_to_aptos_mempool(
		&mut self,
		transaction: SignedTransaction,
	) -> Result<SubmissionStatus, Error> {
		// Check whether the account is whitelisted
		if !self.is_whitelisted(&transaction.sender())? {
			return Ok((MempoolStatus::new(MempoolStatusCode::TooManyTransactions), None));
		}

		// For now, we are going to consider a transaction in flight until it exits the mempool and is sent to the DA as is indicated by WriteBatch.
		let in_flight = {
			let transactions_in_flight = self.transactions_in_flight.read().unwrap();
			transactions_in_flight.get_count()
		};
		info!(
			target: "movement_timing",
			in_flight = %in_flight,
			"transactions_in_flight"
		);
		if let Some(inflight_limit) = self.in_flight_limit {
			if in_flight >= inflight_limit {
				info!(
					target: "movement_timing",
					"shedding_load"
				);
				let status = MempoolStatus::new(MempoolStatusCode::MempoolIsFull);
				return Ok((status, None));
			}
		}

		// Pre-execute Tx to validate its content.
		// Re-create the validator for each Tx because it uses a frozen version of the ledger.
		let vm_validator = VMValidator::new(Arc::clone(&self.db_reader));
		let tx_result = vm_validator.validate_transaction(transaction.clone())?;
		// invert the application priority with the u64 max minus the score from aptos (which is high to low)
		let ranking_score = tx_result.score();
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

		// Add the txn for future validation
		let state_view = self
			.db_reader
			.latest_state_checkpoint_view()
			.expect("Failed to get latest state checkpoint view.");
		let db_seq_num = get_account_sequence_number(&state_view, transaction.sender())?;
		info!(
			tx_sender = %transaction.sender(),
			db_seq_num = %db_seq_num,
			tx_seq_num = %transaction.sequence_number(),
		);
		let tx_hash = transaction.committed_hash();
		let status = self.core_mempool.add_txn(
			transaction,
			ranking_score,
			db_seq_num, //std::cmp::min(*db_seq_num + 1, sequence_number),
			TimelineState::NonQualified,
			true,
		);

		match status.code {
			MempoolStatusCode::Accepted => {
				let now = chrono::Utc::now().timestamp_millis() as u64;
				debug!(%tx_hash, "transaction accepted");
				// increment transactions in flight
				{
					let mut transactions_in_flight = self.transactions_in_flight.write().unwrap();
					transactions_in_flight.increment(now, 1);
				}
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

	use crate::executor::EXECUTOR_CHANNEL_SIZE;
	use aptos_sdk::types::vm_status::DiscardedVMStatus;
	use aptos_storage_interface::state_view::LatestDbStateCheckpointView;
	use aptos_vm_validator::vm_validator;
	use futures::SinkExt;
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
	use maptos_execution_util::config::chain::Config;
	use tempfile::TempDir;

	async fn setup() -> (Context, TransactionPipe, mpsc::Receiver<(u64, SignedTransaction)>, TempDir)
	{
		let (tx_sender, tx_receiver) = mpsc::channel(16);
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			futures_mpsc::channel::<Vec<TxExecutionResult>>(EXECUTOR_CHANNEL_SIZE);

		let (executor, tempdir) =
			Executor::try_test_default(GENESIS_KEYPAIR.0.clone(), mempool_tx_exec_result_sender)
				.await
				.unwrap();
		let (context, background) =
			executor.background(tx_sender, mempool_commit_tx_receiver).unwrap();
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
		let (context, mut transaction_pipe, mut tx_receiver, _tempdir) = setup().await;
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
		assert_eq!(received_transaction.1, user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_cancellation() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (context, mut transaction_pipe, _tx_receiver, _tempdir) = setup().await;
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
		let (context, mut transaction_pipe, mut tx_receiver, _tempdir) = setup().await;
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
		assert_eq!(received_transaction.1, user_transaction);

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
		let (context, mut transaction_pipe, mut tx_receiver, _tempdir) = setup().await;
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
		assert_eq!(received_transaction.1, comparison_user_transaction);

		mempool_handle.abort();

		Ok(())
	}

	#[tokio::test]
	async fn test_repeated_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let (context, mut transaction_pipe, mut tx_receiver, _tempdir) = setup().await;
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
			let bcs_received_transaction = bcs::to_bytes(&received_transaction.1)?;
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
		let (_context, mut transaction_pipe, _tx_receiver, _tempdir) = setup().await;

		// submit a transaction with a valid sequence number
		let user_transaction = create_signed_transaction(0, &maptos_config);
		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit a transaction with a sequence number that is too new
		let user_transaction = create_signed_transaction(34, &maptos_config);
		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::InvalidSeqNumber);

		// submit one signed transaction with a sequence number that is too new for the vm but not for the mempool
		let user_transaction = create_signed_transaction(5, &maptos_config);
		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit a transaction with the same sequence number as the previous one
		let user_transaction = create_signed_transaction(5, &maptos_config);
		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::InvalidSeqNumber);

		Ok(())
	}

	#[tokio::test]
	async fn test_sequence_number_too_old() -> Result<(), anyhow::Error> {
		let (tx_sender, _tx_receiver) = mpsc::channel(16);

		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			futures_mpsc::channel::<Vec<TxExecutionResult>>(EXECUTOR_CHANNEL_SIZE);

		let (mut executor, _tempdir) =
			Executor::try_test_default(GENESIS_KEYPAIR.0.clone(), mempool_tx_exec_result_sender)
				.await?;
		let (context, background) = executor.background(tx_sender, mempool_commit_tx_receiver)?;
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
