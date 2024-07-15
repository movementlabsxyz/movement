use super::Executor;
use aptos_mempool::SubmissionStatus;
use aptos_mempool::{core_mempool::TimelineState, MempoolClientRequest};
use aptos_sdk::types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::SignedTransaction;
use aptos_vm_validator::vm_validator::TransactionValidation;
use aptos_vm_validator::vm_validator::VMValidator;
use futures::StreamExt;
use std::sync::Arc;

impl Executor {
	/// Ticks the transaction reader.
	pub async fn tick_transaction_reader(
		&self,
		transaction_channel: async_channel::Sender<SignedTransaction>,
	) -> Result<(), anyhow::Error> {
		let mut mempool_client_receiver = self.mempool_client_receiver.write().await;
		for _ in 0..256 {
			// use select to safely timeout a request for a transaction without risking dropping the transaction
			// !warn: this may still be unsafe
			tokio::select! {
				_ = tokio::time::sleep(tokio::time::Duration::from_millis(5)) => { () },
				request = mempool_client_receiver.next() => {
					match request {
						Some(request) => {
							match request {
								MempoolClientRequest::SubmitTransaction(transaction, callback) => {
									//preexecute Tx to validate its content.
									//re create the validator for each Tx because it use a frozen version.
									let vm_validator = VMValidator::new(Arc::clone(&self.db.reader));
									let tx_result = vm_validator.validate_transaction(transaction.clone())?;
									//if the verification failed return the error status
									if let Some(vm_status) = tx_result.status() {
										let ms = MempoolStatus::new(MempoolStatusCode::VmError);
										let status: SubmissionStatus = (ms, Some(vm_status));
										callback.send(Ok(status)).map_err(
											|e| anyhow::anyhow!("Error sending callback: {:?}", e)
										)?;
										continue;
									}

									// add to the mempool
									{

										let mut core_mempool = self.core_mempool.write().await;

										let status = core_mempool.add_txn(
											transaction.clone(),
											0,
											transaction.sequence_number(),
											TimelineState::NonQualified,
											true
										);

										match status.code {
											MempoolStatusCode::Accepted => {

											},
											_ => {
												anyhow::bail!("Transaction not accepted: {:?}", status);
											}
										}

										// send along to the receiver
										transaction_channel.send(transaction).await.map_err(
											|e| anyhow::anyhow!("Error sending transaction: {:?}", e)
										)?;

									};

									// report status
									let ms = MempoolStatus::new(MempoolStatusCode::Accepted);
									let status: SubmissionStatus = (ms, None);
									callback.send(Ok(status)).map_err(
										|e| anyhow::anyhow!("Error sending callback: {:?}", e)
									)?;

								},
								MempoolClientRequest::GetTransactionByHash(hash, sender) => {
									let mempool = self.core_mempool.read().await;
									let mempool_result = mempool.get_by_hash(hash);
									sender.send(mempool_result).map_err(
										|e| anyhow::anyhow!("Error sending callback: {:?}", e)
									)?;
								},
							}
						},
						None => {
							break;
						}
					}
				}
			}
		}

		Ok(())
	}

	/// Pipes a batch of transactions from the mempool to the transaction channel.
	/// todo: it may be wise to move the batching logic up a level to the consuming structs.
	pub async fn tick_transaction_pipe(
		&self,
		transaction_channel: async_channel::Sender<SignedTransaction>,
	) -> Result<(), anyhow::Error> {
		self.tick_transaction_reader(transaction_channel.clone()).await?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use std::collections::BTreeSet;

	use super::*;
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		PrivateKey, Uniform,
	};
	use aptos_sdk::types::{AccountKey, LocalAccount};
	use aptos_types::{
		account_address::AccountAddress,
		chain_id::ChainId,
		transaction::{RawTransaction, Script, SignedTransaction, TransactionPayload},
	};
	use futures::channel::oneshot;
	use futures::SinkExt;
	use maptos_execution_util::config::Config;

	fn create_signed_transaction(gas_unit_price: u64, chain_id: ChainId) -> SignedTransaction {
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let public_key = private_key.public_key();
		let transaction_payload = TransactionPayload::Script(Script::new(vec![0], vec![], vec![]));
		let raw_transaction = RawTransaction::new(
			AccountAddress::random(),
			0,
			transaction_payload,
			0,
			gas_unit_price,
			0,
			chain_id, // This is the value used in aptos testing code.
		);
		SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
	}

	#[tokio::test]
	async fn test_pipe_mempool() -> Result<(), anyhow::Error> {
		// header
		let mut executor = Executor::try_test_default()?;
		let user_transaction =
			create_signed_transaction(0, executor.maptos_config.chain.maptos_chain_id.clone());

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		executor
			.mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		let (tx, rx) = async_channel::unbounded();
		executor.tick_transaction_pipe(tx).await?;

		// receive the callback
		callback.await??;

		// receive the transaction
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, user_transaction);

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let mut executor = Executor::try_test_default()?;
		let mempool_executor = executor.clone();

		let (tx, rx) = async_channel::unbounded();
		let mempool_handle = tokio::spawn(async move {
			loop {
				mempool_executor.tick_transaction_pipe(tx.clone()).await?;
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			}
			Ok(()) as Result<(), anyhow::Error>
		});

		let api = executor.get_apis();
		let user_transaction =
			create_signed_transaction(0, executor.maptos_config.chain.maptos_chain_id.clone());
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
		let mut executor = Executor::try_test_default()?;
		let mempool_executor = executor.clone();

		let (tx, rx) = async_channel::unbounded();
		let mempool_handle = tokio::spawn(async move {
			loop {
				mempool_executor.tick_transaction_pipe(tx.clone()).await?;
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			}
			Ok(()) as Result<(), anyhow::Error>
		});

		let api = executor.get_apis();
		let mut user_transactions = BTreeSet::new();
		let mut comparison_user_transactions = BTreeSet::new();
		for _ in 0..25 {
			let user_transaction =
				create_signed_transaction(0, executor.maptos_config.chain.maptos_chain_id.clone());
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
