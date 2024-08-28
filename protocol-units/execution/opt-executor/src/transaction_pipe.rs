//! Task processing incoming transactions for the opt API.

use aptos_config::config::NodeConfig;
use aptos_mempool::core_mempool::CoreMempool;
use aptos_mempool::SubmissionStatus;
use aptos_mempool::{core_mempool::TimelineState, MempoolClientRequest};
use aptos_sdk::types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_storage_interface::DbReader;
use aptos_types::transaction::SignedTransaction;
use aptos_vm_validator::vm_validator::TransactionValidation;
use aptos_vm_validator::vm_validator::VMValidator;

use futures::channel::mpsc as futures_mpsc;
use futures::StreamExt;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, info, info_span, warn, Instrument};

use std::sync::{atomic::AtomicU64, Arc};
use std::time::{Duration, Instant};

const GC_INTERVAL: Duration = Duration::from_secs(30);

/// Domain error for the transaction pipe task
#[derive(Debug, Clone, Error)]
pub enum Error {
	#[error("Transaction Pipe InternalError: {0}")]
	InternalError(String),
	#[error("Transaction not accepted: {0}")]
	TransactionNotAccepted(MempoolStatus),
	#[error("Transaction stream closed")]
	InputClosed,
}

impl From<anyhow::Error> for Error {
	fn from(e: anyhow::Error) -> Self {
		Error::InternalError(e.to_string())
	}
}

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
}

impl TransactionPipe {
	pub(crate) fn new(
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		transaction_sender: mpsc::Sender<SignedTransaction>,
		db_reader: Arc<dyn DbReader>,
		node_config: &NodeConfig,
		transactions_in_flight: Arc<AtomicU64>,
		transactions_in_flight_limit: u64,
	) -> Self {
		TransactionPipe {
			mempool_client_receiver,
			transaction_sender,
			db_reader,
			core_mempool: CoreMempool::new(node_config),
			transactions_in_flight,
			in_flight_limit: transactions_in_flight_limit,
			last_gc: Instant::now(),
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
		}

		if self.last_gc.elapsed() >= GC_INTERVAL {
			self.core_mempool.gc();
			self.last_gc = Instant::now();
		}

		Ok(())
	}

	async fn submit_transaction(
		&mut self,
		transaction: SignedTransaction,
	) -> Result<SubmissionStatus, Error> {
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
				return Ok((ms, tx_result.status()));
			}
			None => {}
		}

		let sequence_number = transaction.sequence_number();
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
				self.transaction_sender
					.send(transaction)
					.await
					.map_err(|e| anyhow::anyhow!("Error sending transaction: {:?}", e))?;
				// increment transactions in flight
				self.transactions_in_flight.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
				self.core_mempool.commit_transaction(&sender, sequence_number);
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
	use crate::{Executor, Service};
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
	use aptos_mempool::MempoolClientSender;
	use aptos_types::{
		account_config, test_helpers::transaction_test_helpers, transaction::SignedTransaction,
	};
	use aptos_vm_genesis::GENESIS_KEYPAIR;
	use futures::channel::oneshot;
	use futures::SinkExt;
	use maptos_execution_util::config::chain::Config;

	fn setup() -> (TransactionPipe, MempoolClientSender, mpsc::Receiver<SignedTransaction>) {
		let (tx_sender, tx_receiver) = mpsc::channel(16);
		let (executor, config, _tempdir) =
			Executor::try_test_default(GENESIS_KEYPAIR.0.clone()).unwrap();
		let (context, transaction_pipe, _indexer_runtime) =
			executor.background(tx_sender, &config).unwrap();
		(transaction_pipe, context.mempool_client_sender(), tx_receiver)
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
		let (mut transaction_pipe, mut mempool_client_sender, mut tx_receiver) = setup();
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
		let received_transaction = tx_receiver.recv().await.unwrap();
		assert_eq!(received_transaction, user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_cancellation() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (mut transaction_pipe, mut mempool_client_sender, _tx_receiver) = setup();
		let user_transaction = create_signed_transaction(1, &maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		mempool_client_sender
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
		let (mut transaction_pipe, mut mempool_client_sender, mut tx_receiver) = setup();
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
		let received_transaction = tx_receiver.recv().await.unwrap();
		assert_eq!(received_transaction, user_transaction);

		// send the same transaction again
		let (req_sender, callback) = oneshot::channel();
		mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		transaction_pipe.tick().await?;

		callback.await??;

		let received_transaction = tx_receiver.recv().await.unwrap();
		assert_eq!(received_transaction, user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let (tx_sender, mut tx_receiver) = mpsc::channel(16);
		let (executor, config, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let (context, mut transaction_pipe, _indexer_runtime) =
			executor.background(tx_sender, &config)?;
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
		let (tx_sender, mut tx_receiver) = mpsc::channel(16);
		let (executor, config, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let (context, mut transaction_pipe, _indexer_runtime) =
			executor.background(tx_sender, &config)?;
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
}
