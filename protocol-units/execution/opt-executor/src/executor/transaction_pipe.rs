use super::Executor;
use aptos_mempool::{core_mempool::TimelineState, MempoolClientRequest};
use aptos_sdk::types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::SignedTransaction;
use futures::StreamExt;
use thiserror::Error;
use tracing::{info, warn, debug, info_span, Instrument};
use aptos_mempool::core_mempool::CoreMempool;
use aptos_mempool::SubmissionStatus;
use async_channel::Sender;

#[derive(Debug, Clone, Error)]
pub enum TransactionPipeError {
	#[error("Transaction Pipe InternalError: {0}")]
	InternalError(String),
	#[error("Transaction not accepted: {0}")]
	TransactionNotAccepted(MempoolStatus),
	#[error("Transaction stream closed")]
	InputClosed,
}

impl From<anyhow::Error> for TransactionPipeError {
	fn from(e: anyhow::Error) -> Self {
		TransactionPipeError::InternalError(e.to_string())
	}
}

impl Executor {

	/// Pipes a batch of transactions from the mempool to the transaction channel.
	/// todo: it may be wise to move the batching logic up a level to the consuming structs.
	pub async fn tick_transaction_pipe(
		&self,
		core_mempool: &mut CoreMempool,
		transaction_channel: async_channel::Sender<SignedTransaction>,
		last_gc: &mut std::time::Instant,
	) -> Result<(), TransactionPipeError> {
		// Drop the receiver RwLock as soon as possible.
		let next = {
			let mut mempool_client_receiver = self.mempool_client_receiver.write().await;
			mempool_client_receiver.next().await
		};
		if let Some(request) = next {
			match request {
				MempoolClientRequest::SubmitTransaction(transaction, callback) => {
					// For now, we are going to consider a transaction in flight until it exits the mempool and is sent to the DA as is indicated by WriteBatch.
					let in_flight = self.transactions_in_flight.load(std::sync::atomic::Ordering::SeqCst);
					info!(
						target: "movement_timing",
						in_flight = %in_flight,
						"transactions_in_flight"
					);
					if in_flight > 2u64.pow(16) {
						info!(
							target: "movement_timing",
							"shedding_load"
						);
						let status = MempoolStatus::new(MempoolStatusCode::MempoolIsFull);
						callback.send(Ok((status.clone(), None))).map_err(
							|e| TransactionPipeError::InternalError(format!("Error sending transaction: {:?}", e))
						)?;
						return Ok(())
					}

					let span = info_span!(
						target: "movement_timing",
						"submit_transaction",
						tx_hash = %transaction.committed_hash(),
						sender = %transaction.sender(),
						sequence_number = transaction.sequence_number(),
					);
					let status = self
						.submit_transaction(core_mempool, transaction, transaction_channel)
						.instrument(span)
						.await?;

					callback.send(Ok(status)).map_err(
						|e| TransactionPipeError::InternalError(format!("Error sending transaction: {:?}", e))
					)?;

				}
				MempoolClientRequest::GetTransactionByHash(hash, sender) => {
					let mempool_result = core_mempool.get_by_hash(hash);
					sender.send(mempool_result).map_err(
						|e| TransactionPipeError::InternalError(format!("Error sending transaction: {:?}", e))
					)?;
				}
			}
		}

		if last_gc.elapsed().as_secs() > 30 {
			core_mempool.gc();
			*last_gc = std::time::Instant::now();
		}

		Ok(())
	}

	async fn submit_transaction(
		&self,
		core_mempool: &mut CoreMempool,
		transaction: SignedTransaction,
		transaction_channel: Sender<SignedTransaction>,
	) -> Result<SubmissionStatus, TransactionPipeError> {
		// Pre-execute Tx to validate its content.
		// Re-create the validator for each Tx because it uses a frozen version of the ledger.
		// let vm_validator = VMValidator::new(Arc::clone(&self.db.reader));
		// let tx_result = vm_validator.validate_transaction(transaction.clone())?;

		// let status = if let Some(vm_status) = tx_result.status() {
		// 	// If the verification failed, return the error status.
		// 	let ms = MempoolStatus::new(MempoolStatusCode::VmError);
		// 	(ms, Some(vm_status))
		// } else {

		debug!(
			"Adding transaction to mempool: {:?} {:?}",
			transaction,
			transaction.sequence_number()
		);
		let status = core_mempool.add_txn(
			transaction.clone(),
			0,
			transaction.sequence_number(),
			TimelineState::NonQualified,
			true,
		);

		match status.code {
			MempoolStatusCode::Accepted => {
				debug!("Transaction accepted: {:?}", transaction);
				transaction_channel
					.send(transaction)
					.await
					.map_err(|e| anyhow::anyhow!("Error sending transaction: {:?}", e))?;
				// increment transactions in flight
				self.transactions_in_flight.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
			}
			_ => {
				warn!("Transaction not accepted: {:?}", status);
			}
		}

		// report status
		let ms = MempoolStatus::new(MempoolStatusCode::Accepted);
		Ok((ms, None))
	}
}

#[cfg(test)]
mod tests {

	use std::collections::BTreeSet;

	use super::*;
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
	use aptos_types::{
		account_config, test_helpers::transaction_test_helpers, transaction::SignedTransaction,
	};
	use aptos_vm_genesis::GENESIS_KEYPAIR;
	use futures::channel::oneshot;
	use futures::SinkExt;
	use maptos_execution_util::config::Config;

	fn create_signed_transaction(
		sequence_number: u64,
		maptos_config: &Config,
	) -> SignedTransaction {
		let address = account_config::aptos_test_root_address();
		transaction_test_helpers::get_test_txn_with_chain_id(
			address,
			sequence_number,
			&GENESIS_KEYPAIR.0,
			GENESIS_KEYPAIR.1.clone(),
			maptos_config.chain.maptos_chain_id.clone(), // This is the value used in aptos testing code.
		)
	}

	#[tokio::test]
	async fn test_pipe_mempool() -> Result<(), anyhow::Error> {
		// header
		let (mut executor, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let user_transaction = create_signed_transaction(1, &executor.maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		executor
			.mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		let (tx, rx) = async_channel::unbounded();
		let mut core_mempool = CoreMempool::new(&executor.node_config.clone());
		executor.tick_transaction_pipe(&mut core_mempool, tx, &mut std::time::Instant::now()).await?;

		// receive the callback
		let (status, _vm_status_code) = callback.await??;
		assert_eq!(status.code, MempoolStatusCode::Accepted);

		// receive the transaction
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_cancellation() -> Result<(), anyhow::Error> {
		// header
		let (mut executor, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let user_transaction = create_signed_transaction(1, &executor.maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		executor
			.mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// drop the callback to simulate cancellation of the request
		drop(callback);

		// tick the transaction pipe, should succeed
		let (tx, rx) = async_channel::unbounded();
		let mut core_mempool = CoreMempool::new(&executor.node_config.clone());
		executor.tick_transaction_pipe(&mut core_mempool, tx, &mut std::time::Instant::now()).await?;

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_with_malformed_transaction() -> Result<(), anyhow::Error> {
		// header
		let (mut executor, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let user_transaction = create_signed_transaction(1, &executor.maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		executor
			.mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		let (tx, rx) = async_channel::unbounded();
		let mut core_mempool = CoreMempool::new(&executor.node_config.clone());
		executor.tick_transaction_pipe(&mut core_mempool, tx.clone(), &mut std::time::Instant::now()).await?;

		// receive the callback
		let (status, _vm_status_code) = callback.await??;
		// dbg!(_vm_status_code);
		assert_eq!(status.code, MempoolStatusCode::Accepted);

		// receive the transaction
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, user_transaction);

		// send the same transaction again
		let (req_sender, callback) = oneshot::channel();
		executor
			.mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		let (tx, rx) = async_channel::unbounded();
		let mut core_mempool = CoreMempool::new(&executor.node_config.clone());
		executor.tick_transaction_pipe(&mut core_mempool, tx, &mut std::time::Instant::now()).await?;
		/*match executor.tick_transaction_pipe(tx).await {
			Err(TransactionPipeError::TransactionNotAccepted(_)) => {}
			Err(e) => return Err(anyhow::anyhow!("Unexpected error: {:?}", e)),
			Ok(_) => return Err(anyhow::anyhow!("Expected error")),
		}*/

		callback.await??;

		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let (executor, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let mempool_executor = executor.clone();

		let (tx, rx) = async_channel::unbounded();
		let mempool_handle = tokio::spawn(async move {
			let mut core_mempool = CoreMempool::new(&mempool_executor.node_config.clone());
			loop {
				mempool_executor.tick_transaction_pipe(&mut core_mempool, tx.clone(), &mut std::time::Instant::now()).await?;
			}
			Ok(()) as Result<(), anyhow::Error>
		});

		let api = executor.get_apis();
		let user_transaction = create_signed_transaction(1, &executor.maptos_config);
		let comparison_user_transaction = user_transaction.clone();
		let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;
		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, comparison_user_transaction);

		mempool_handle.abort();

		Ok(())
	}

	#[tokio::test]
	async fn test_repeated_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let (executor, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let mempool_executor = executor.clone();

		let (tx, rx) = async_channel::unbounded();
		let mempool_handle = tokio::spawn(async move {
			let mut core_mempool = CoreMempool::new(&mempool_executor.node_config.clone());
			loop {
				mempool_executor.tick_transaction_pipe(&mut core_mempool, tx.clone(), &mut std::time::Instant::now()).await?;
			}
			Ok(()) as Result<(), anyhow::Error>
		});

		let api = executor.get_apis();
		let mut user_transactions = BTreeSet::new();
		let mut comparison_user_transactions = BTreeSet::new();
		for i in 1..25 {
			let user_transaction = create_signed_transaction(i, &executor.maptos_config);
			let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;
			user_transactions.insert(bcs_user_transaction.clone());

			let request =
				SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
			api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

			let received_transaction = rx.recv().await?;
			let bcs_received_transaction = bcs::to_bytes(&received_transaction)?;
			comparison_user_transactions.insert(bcs_received_transaction.clone());
		}

		assert_eq!(user_transactions.len(), comparison_user_transactions.len());
		assert_eq!(user_transactions, comparison_user_transactions);

		mempool_handle.abort();

		Ok(())
	}
}
