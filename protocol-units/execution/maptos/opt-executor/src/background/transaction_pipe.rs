//! Task processing incoming transactions for the opt API.
use super::Error;
use crate::executor::TxExecutionResult;
use crate::gc_account_sequence_number::UsedSequenceNumberPool;
use aptos_account_whitelist::config::Config as WhitelistConfig;
use aptos_config::config::NodeConfig;
use aptos_mempool::{
	core_mempool::{CoreMempool, TimelineState},
	MempoolClientRequest, SubmissionStatus,
};
use aptos_storage_interface::{state_view::LatestDbStateCheckpointView, DbReader};
use aptos_types::{
	account_address::AccountAddress,
	mempool_status::{MempoolStatus, MempoolStatusCode},
	transaction::{SignedTransaction, TransactionStatus},
};
use aptos_vm_validator::vm_validator::{
	get_account_sequence_number, TransactionValidation, VMValidator,
};
use bcs;
use futures::channel::mpsc as futures_mpsc;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use maptos_execution_util::config::mempool::Config as MempoolConfig;
use movement_collections::garbage::counted::GcCounter;
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_proto::BatchWriteRequest;
use movement_signer::cryptography::ed25519::Ed25519;
use movement_signer_loader::identifiers::SignerIdentifier;
use movement_signer_loader::{Load, LoadedSigner};
use movement_types::transaction::Transaction;
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{debug, info, info_span, warn, Instrument};

const GC_INTERVAL: Duration = Duration::from_secs(30);
const MEMPOOL_INTERVAL: Duration = Duration::from_millis(240); // this is based on slot times and global TCP RTT, essentially we expect to collect all transactions sent in the same slot in around 240ms

pub struct TransactionPipe {
	// The receiver for Tx execution to commit in the mempool.
	mempool_commit_tx_receiver: futures_mpsc::Receiver<Vec<TxExecutionResult>>,
	// The receiver for the mempool client.
	mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
	// Access to the ledger DB. TODO: reuse an instance of VMValidator
	db_reader: Arc<dyn DbReader>,
	// State of the Aptos mempool
	core_mempool: CoreMempool,
	// Shared reference on the counter of transactions in flight.
	transactions_in_flight: Arc<RwLock<GcCounter>>,
	// The configured limit on transactions in flight
	in_flight_limit: Option<u64>,
	// Timestamp of the last garbage collection
	last_gc: Instant,
	// The pool of used sequence numbers
	used_sequence_number_pool: UsedSequenceNumberPool,
	/// The accounts whitelisted for ingress
	whitelisted_accounts: Option<HashSet<AccountAddress>>,
	/// Batch signer
	da_batch_signer: SignerIdentifier,
	/// Mempool configuration from maptos_execution.
	mempool_config: MempoolConfig,
}

enum SequenceNumberValidity {
	Valid(u64),
	Invalid(SubmissionStatus),
}

impl TransactionPipe {
	pub(crate) fn new(
		mempool_commit_tx_receiver: futures_mpsc::Receiver<Vec<TxExecutionResult>>, // Sender, seq number)
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		db_reader: Arc<dyn DbReader>,
		node_config: &NodeConfig,
		mempool_config: &MempoolConfig,
		whitelist_config: &WhitelistConfig,
		transactions_in_flight: Arc<RwLock<GcCounter>>,
		transactions_in_flight_limit: Option<u64>,
		da_batch_signer: SignerIdentifier,
	) -> Result<Self, anyhow::Error> {
		let whitelisted_accounts = whitelist_config.whitelisted_accounts()?;
		info!("Whitelisted accounts: {:?}", whitelisted_accounts);

		Ok(TransactionPipe {
			mempool_commit_tx_receiver,
			mempool_client_receiver,
			db_reader,
			core_mempool: CoreMempool::new(node_config),
			transactions_in_flight,
			in_flight_limit: transactions_in_flight_limit,
			last_gc: Instant::now(),
			used_sequence_number_pool: UsedSequenceNumberPool::new(
				mempool_config.sequence_number_ttl_ms,
				mempool_config.gc_slot_duration_ms,
			),
			whitelisted_accounts,
			da_batch_signer,
			mempool_config: mempool_config.clone(),
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

	pub async fn run(mut self, da_client: (impl DaSequencerClient + 'static)) -> Result<(), Error> {
		let mut build_batch_interval = tokio::time::interval(MEMPOOL_INTERVAL);
		let mut mempool_gc_interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
		let mut sent_batch_futures = FuturesUnordered::new();

		loop {
			tokio::select! {
				Some(request) = self.mempool_client_receiver.next() => {
					self.tick_requests(request).await?;
				}
				Some(batches) = self.mempool_commit_tx_receiver.next() => {
					self.tick_commit_tx(batches).await?;
				}
				_ = build_batch_interval.tick() => {
					let batch_jh = self.tick_mempool_sender(&da_client).await?;
					if let Some(jh) = batch_jh {
						sent_batch_futures.push(jh);
					}
				}
				_ = mempool_gc_interval.tick() => {
					self.tick_gc();
				}

				Some(result) = sent_batch_futures.next() => {
					match result {
						Ok(Ok(response)) => {
							if !response.answer {
								tracing::error!("DA Sequencer reject batch.");
								panic!("DA Sequencer reject batch., can't send batch, exit process");
							}
						}
						Ok(Err(err)) => {
							tracing::error!("Send batch to Da failed because of a connection issue: {err}");
							//TODO put some reconnection for now panic.
							panic!("DA connection failed, can't send batch, exit process");
						}
						Err(err) => {
							tracing::error!("Tokio send batch task execution failed: {err}");
							//TODO see what consequence of this error.
						}
					}
				}
			}
		}
	}

	/// Pipes a batch of transactions from the executor to commit then in the Aptos mempool.
	pub(crate) async fn tick_commit_tx(
		&mut self,
		batches: Vec<TxExecutionResult>,
	) -> Result<(), Error> {
		for tx_result in batches {
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
		Ok(())
	}

	/// Pipes a batch of transactions from the mempool to the transaction channel.
	pub async fn tick_requests(&mut self, request: MempoolClientRequest) -> Result<(), Error> {
		match request {
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
		}

		Ok(())
	}

	/// Extracts a batch of transactions from the mempool and sends it to the DA.
	pub(crate) async fn tick_mempool_sender(
		&mut self,
		da_client: &(impl DaSequencerClient + 'static),
	) -> Result<
		Option<JoinHandle<Result<movement_da_sequencer_proto::BatchWriteResponse, tonic::Status>>>,
		Error,
	> {
		let transactions = self.core_mempool.get_batch_with_ranking_score(
			self.mempool_config.max_tx_per_batch,
			self.mempool_config.max_batch_size,
			true,            // allow partial batches
			BTreeMap::new(), // exclude Tx
		);

		debug!("Create a batch of {} transactions and submit it.", transactions.len());

		let batch: Vec<Transaction> = transactions
			.into_iter()
			.map(|(transaction, ranking_score)| {
				let priority = u64::MAX - ranking_score;
				let sender = transaction.sender();
				let seq = transaction.sequence_number();

				self.core_mempool.commit_transaction(&sender, seq);
				debug!(
					target: "movement_timing",
					tx_hash = %transaction.committed_hash(),
					sender = %transaction.sender(),
					sequence_number = transaction.sequence_number(),
					"Tx build batch add transaction",
				);
				bcs::to_bytes(&transaction).map(|serialized| {
					Transaction::new(serialized, priority, transaction.sequence_number())
				})
			})
			.collect::<Result<Vec<_>, _>>()?;

		if !batch.is_empty() {
			// Build batch and submit request.
			let batch_bytes = bcs::to_bytes(&batch).expect("Serialization failed");
			let loader: LoadedSigner<Ed25519> = self.da_batch_signer.load().await?;
			let encoded =
				movement_da_sequencer_client::sign_and_encode_batch(batch_bytes, &loader).await?;
			//send the batch in a separate task to avoid to slow the loop.
			let handle = tokio::spawn({
				let mut client = da_client.clone();
				async move { client.batch_write(BatchWriteRequest { data: encoded }).await }
			});
			Ok(Some(handle))
		} else {
			Ok(None)
		}
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

		info!(
			tx_hash = %tx_hash,
			status = %status,
			"Transaction added to the mempool with status"
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
	use futures::stream;
	use futures::SinkExt;
	use movement_da_sequencer_client::ClientDaSequencerError;
	use movement_da_sequencer_client::StreamReadBlockFromHeight;
	use movement_da_sequencer_proto::BatchWriteResponse;
	use movement_da_sequencer_proto::Blockv1;
	use std::collections::BTreeSet;
	use std::sync::Mutex;

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
	use movement_types::transaction::Transaction as MvTransaction;
	use tempfile::TempDir;

	async fn setup() -> (Context, TransactionPipe, TempDir) {
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			futures_mpsc::channel::<Vec<TxExecutionResult>>(EXECUTOR_CHANNEL_SIZE);
		let (executor, tempdir) =
			Executor::try_test_default(GENESIS_KEYPAIR.0.clone(), mempool_tx_exec_result_sender)
				.await
				.unwrap();
		let (context, background) = executor.background(mempool_commit_tx_receiver).unwrap();
		let transaction_pipe = background.into_transaction_pipe();
		(context, transaction_pipe, tempdir)
	}

	#[derive(Clone)]
	pub struct TxPipeTestDaSequencerClient {
		pub received_tx: Arc<Mutex<Vec<MvTransaction>>>,
	}

	impl TxPipeTestDaSequencerClient {
		fn new() -> Self {
			TxPipeTestDaSequencerClient { received_tx: Arc::new(Mutex::new(vec![])) }
		}
	}

	impl DaSequencerClient for TxPipeTestDaSequencerClient {
		async fn stream_read_from_height(
			&mut self,
			_request: movement_da_sequencer_proto::StreamReadFromHeightRequest,
		) -> Result<StreamReadBlockFromHeight, ClientDaSequencerError> {
			let never_ending_stream = stream::pending::<Result<Blockv1, ClientDaSequencerError>>();

			Ok(Box::pin(never_ending_stream))
		}

		/// Writes a batch of transactions to the Da Sequencer node
		async fn batch_write(
			&mut self,
			request: movement_da_sequencer_proto::BatchWriteRequest,
		) -> Result<movement_da_sequencer_proto::BatchWriteResponse, tonic::Status> {
			tracing::info!("TxPipeTestDaSequencerClient receive a batch");
			let batch_data = request.data;
			let batch: Vec<MvTransaction> =
				match movement_da_sequencer_client::deserialize_full_node_batch(batch_data)
					.map_err(|_| "bcs deserialisation error.".to_string())
					.and_then(|(_, _, bytes)| {
						bcs::from_bytes(&bytes)
							.map_err(|_| "bcs deserialisation error.".to_string())
					}) {
					Ok(batch) => batch,
					Err(err) => {
						tracing::warn!(error = %err, "Invalid batch send, verification / validation failed.");
						return Ok(BatchWriteResponse { answer: false });
					}
				};
			tracing::info!("TxPipeTestDaSequencerClient that contains {} Tx", batch.len());
			batch.into_iter().for_each(|tx| self.received_tx.lock().unwrap().push(tx));
			Ok(BatchWriteResponse { answer: true })
		}
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
	async fn test_pipe_mempool_one_tx() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (context, transaction_pipe, _tempdir) = setup().await;

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone()));

		let user_transaction = create_signed_transaction(0, &maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		context
			.mempool_client_sender()
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// receive the callback
		let (status, _vm_status_code) = callback.await??;
		assert_eq!(status.code, MempoolStatusCode::Accepted);

		//wait Tx propagation
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		//validate teh Tx has been sent to the DA
		{
			let txs = da_client.received_tx.lock().unwrap();
			assert_eq!(txs.len(), 1);
			assert_eq!(txs.get(0).unwrap().sequence_number(), user_transaction.sequence_number());
		}

		mempool_handle.abort();
		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_cancellation() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (context, transaction_pipe, _tempdir) = setup().await;
		let user_transaction = create_signed_transaction(0, &maptos_config);

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone()));

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		context
			.mempool_client_sender()
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// drop the callback to simulate cancellation of the request
		drop(callback);

		// The Tx should be accepted and sent. wait Tx propagation
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		//validate teh Tx has been sent to the DA
		{
			let txs = da_client.received_tx.lock().unwrap();
			assert_eq!(txs.len(), 1);
			assert_eq!(txs.get(0).unwrap().sequence_number(), user_transaction.sequence_number());
		}

		mempool_handle.abort();
		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_with_duplicate_transaction() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (context, mut transaction_pipe, _tempdir) = setup().await;
		let mut mempool_client_sender = context.mempool_client_sender();
		let user_transaction = create_signed_transaction(0, &maptos_config);

		// Add 2 time the transaction to mempool
		let (mempool_status, _) = transaction_pipe
			.add_transaction_to_aptos_mempool(user_transaction.clone())
			.await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone()));

		//wait Tx propagation
		// Verify the Tx is added only one time
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		//validate teh Tx has been sent to the DA
		{
			let txs = da_client.received_tx.lock().unwrap();
			assert_eq!(txs.len(), 1);
		}

		mempool_handle.abort();

		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let (context, transaction_pipe, _tempdir) = setup().await;
		let service = Service::new(&context);

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone()));

		let api = service.get_apis();
		let user_transaction = create_signed_transaction(0, &context.config().chain);
		//let comparison_user_transaction = user_transaction.clone();
		let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;
		let request = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		//wait Tx propagation
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		//validate teh Tx has been sent to the DA
		{
			let txs = da_client.received_tx.lock().unwrap();
			assert_eq!(txs.len(), 1);
			assert_eq!(txs.get(0).unwrap().sequence_number(), user_transaction.sequence_number());
		}

		mempool_handle.abort();

		Ok(())
	}

	// Ignore test because with the new Aptos core mempool, Tx with a seqnumber too higher from the db one stay in it.
	// Need to execute block to update ledger db or use several account.
	// This test should be done in the e2e test.
	#[tokio::test]
	#[ignore]
	async fn test_repeated_pipe_mempool_from_api() -> Result<(), anyhow::Error> {
		let (context, transaction_pipe, _tempdir) = setup().await;
		let service = Service::new(&context);

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone()));

		let api = service.get_apis();
		let mut user_transactions: BTreeSet<Vec<u8>> = BTreeSet::new();
		//let mut comparison_user_transactions: BTreeSet<Vec<u8>> = BTreeSet::new();
		for i in 0..25 {
			let user_transaction = create_signed_transaction(i, &context.config().chain);
			let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;
			user_transactions.insert(bcs_user_transaction.clone());

			let request =
				SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(bcs_user_transaction));
			api.transactions.submit_transaction(AcceptType::Bcs, request).await?;
		}

		//wait Tx propagation
		let _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
		//validate teh Tx has been sent to the DA
		{
			let txs = da_client.received_tx.lock().unwrap();
			assert_eq!(txs.len(), 25);
		}

		mempool_handle.abort();

		Ok(())
	}

	#[tokio::test]
	async fn test_submit_with_different_seqnumber() -> Result<(), anyhow::Error> {
		// set up
		let maptos_config = Config::default();
		let (_context, mut transaction_pipe, _tempdir) = setup().await;

		// submit a transaction with a valid sequence number
		let user_transaction = create_signed_transaction(0, &maptos_config);
		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit a transaction with a sequence number that is too new
		let user_transaction = create_signed_transaction(34, &maptos_config);
		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit one signed transaction with a sequence number that is too new for the vm but not for the mempool
		let user_transaction = create_signed_transaction(5, &maptos_config);
		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit a transaction with the same sequence number as the previous one
		let user_transaction = create_signed_transaction(5, &maptos_config);
		let (mempool_status, _) =
			transaction_pipe.add_transaction_to_aptos_mempool(user_transaction).await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		Ok(())
	}

	#[tokio::test]
	async fn test_sequence_number_too_old() -> Result<(), anyhow::Error> {
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			futures_mpsc::channel::<Vec<TxExecutionResult>>(EXECUTOR_CHANNEL_SIZE);

		let (mut executor, _tempdir) =
			Executor::try_test_default(GENESIS_KEYPAIR.0.clone(), mempool_tx_exec_result_sender)
				.await?;
		let (context, background) = executor.background(mempool_commit_tx_receiver)?;
		let transaction_pipe = background.into_transaction_pipe();

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client));

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
