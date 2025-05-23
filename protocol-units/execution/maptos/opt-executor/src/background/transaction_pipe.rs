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
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tracing::{debug, info, info_span, warn, Instrument};

const GC_INTERVAL: Duration = Duration::from_secs(30);
const MEMPOOL_INTERVAL: Duration = Duration::from_millis(240); // this is based on slot times and global TCP RTT, essentially we expect to collect all transactions sent in the same slot in around 240ms

pub struct TransactionPipe {
	// The receiver for Tx execution to commit in the mempool.
	mempool_commit_tx_receiver: UnboundedReceiver<Vec<TxExecutionResult>>,
	// The receiver for the mempool client.
	// Access to the ledger DB. TODO: reuse an instance of VMValidator
	db_reader: Arc<dyn DbReader>,
	// State of the Aptos mempool
	core_mempool: Arc<RwLock<CoreMempool>>,
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
	pub fn core_mempool(&self) -> Arc<RwLock<CoreMempool>> {
		self.core_mempool.clone()
	}
	pub fn db_reader(&self) -> Arc<dyn DbReader> {
		self.db_reader.clone()
	}

	pub fn transactions_in_flight(&self) -> (Arc<RwLock<GcCounter>>, Option<u64>) {
		(self.transactions_in_flight.clone(), self.in_flight_limit.clone())
	}

	pub(crate) fn new(
		mempool_commit_tx_receiver: UnboundedReceiver<Vec<TxExecutionResult>>, // Sender, seq number)
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
			db_reader,
			core_mempool: Arc::new(RwLock::new(CoreMempool::new(node_config))),
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

	pub async fn run(
		mut self,
		da_client: (impl DaSequencerClient + 'static + std::marker::Sync),
		mut mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
	) -> Result<(), Error> {
		let mut build_batch_deadline = tokio::time::Instant::now() + MEMPOOL_INTERVAL;
		let mut mempool_gc_interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
		let mut sent_batch_futures = FuturesUnordered::new();

		// Start 2 loops because we manage 2 differentes process.
		// The Tx request stream that can be very fast with a lot of request.
		// And the Tx pipe processing that is more aroung hundred of ms for each sub process.
		// It avoids to starve the second if the first one is to fast.
		// Request loop processing
		let request_jh = tokio::spawn({
			let core_mempool = self.core_mempool();
			let db_reader = self.db_reader();
			let (transactions_in_flight, in_flight_limit) = self.transactions_in_flight();
			let mut counter = 0;
			async move {
				// Process messages received on the channel.
				loop {
					match mempool_client_receiver.next().await {
						Some(request) => {
							TransactionPipe::tick_requests(
								request,
								&core_mempool,
								&db_reader,
								&transactions_in_flight,
								in_flight_limit,
								&mut counter,
							)
							.await?;
						}
						None => {
							//Channel closed end loop
							return Err(Error::InternalError(
								"Mempool Request channel closed. End request processing"
									.to_string(),
							));
						}
					}
				}
			}
		});

		// Tx processing loop
		let pipe_jh = tokio::spawn({
			let core_mempool = self.core_mempool.clone();
			let da_batch_signer = self.da_batch_signer.clone();
			let mempool_config = self.mempool_config.clone();
			async move {
				loop {
					tokio::select! {
						Some(batches) = self.mempool_commit_tx_receiver.recv() => {
							TransactionPipe::tick_commit_tx(&core_mempool, batches).await?;
						}
						_ = tokio::time::sleep_until(build_batch_deadline) => {
							build_batch_deadline = tokio::time::Instant::now() + MEMPOOL_INTERVAL;
							if let Some(jh) = TransactionPipe::tick_mempool_sender(&core_mempool, &da_client, &da_batch_signer, &mempool_config).await? {
								sent_batch_futures.push(jh);
							}
						}
						_ = mempool_gc_interval.tick() => {
							self.tick_gc();
						}

						Some(result) = sent_batch_futures.next() => {
							match result {
								Ok(Ok(response)) => {
									debug!("After sent batch.");
									if !response.answer {
										tracing::error!("DA Sequencer reject batch, can't send batch, exit process");
										return Err(Error::InternalError(format!("DA Sequencer reject batch, can't send batch, exit process")));
									}
								}
								Ok(Err(err)) => {
									tracing::error!("Send batch to Da failed because of a connection issue: {err}, can't send batch, exit process");
										return Err(Error::InternalError(format!("Send batch to Da failed because of a connection issue: {err}, can't send batch, exit process")));
								}
								Err(err) => {
									tracing::error!("Tokio send batch task execution failed: {err}, can't send batch, exit process");
										return Err(Error::InternalError(format!("Tokio send batch task execution failed: {err}, can't send batch, exit process")));
								}
							}
						}
					}
				}
			}
		});

		tokio::select! {
			res = request_jh => {
				tracing::error!("Tx request loop break with error: {res:?}.");
				res.map_err(|err| anyhow::anyhow!("Tx request loop break with error: {err:?}."))?
			}
			res = pipe_jh => {
				tracing::error!("Tx pipe loop break with error: {res:?}.");
				res.map_err(|err| anyhow::anyhow!("Tx pipe loop break with error: {err:?}."))?
			}
		}
	}

	/// Pipes a batch of transactions from the mempool to the transaction channel.
	pub async fn tick_requests(
		request: MempoolClientRequest,
		core_mempool: &Arc<RwLock<CoreMempool>>,
		db_reader: &Arc<dyn DbReader>,
		transactions_in_flight: &Arc<RwLock<GcCounter>>,
		in_flight_limit: Option<u64>,
		counter: &mut u64,
	) -> Result<(), Error> {
		match request {
			MempoolClientRequest::SubmitTransaction(transaction, callback) => {
				let span = info_span!(
					target: "movement_timing",
					"submit_transaction",
					tx_hash = %transaction.committed_hash(),
					sender = %transaction.sender(),
					sequence_number = transaction.sequence_number(),
					expiration_ts = transaction.expiration_timestamp_secs(),
				);
				let status = TransactionPipe::add_transaction_to_aptos_mempool(
					transaction,
					core_mempool,
					db_reader,
					transactions_in_flight,
					in_flight_limit,
				)
				.instrument(span)
				.await?;

				debug!("Sending back Tx status: {status:?} and counter={counter}");
				callback.send(Ok(status)).unwrap_or_else(|_| {
					debug!("SubmitTransaction request canceled");
				});
				*counter = 0;
			}
			MempoolClientRequest::GetTransactionByHash(hash, sender) => {
				let mempool_result = { core_mempool.read().unwrap().get_by_hash(hash) };
				sender.send(mempool_result).unwrap_or_else(|_| {
					info!("GetTransactionByHash request canceled");
				});

				*counter += 1;
			}
		}

		Ok(())
	}

	/// Pipes a batch of transactions from the executor to commit then in the Aptos mempool.
	pub(crate) async fn tick_commit_tx(
		core_mempool: &RwLock<CoreMempool>,
		batches: Vec<TxExecutionResult>,
	) -> Result<(), Error> {
		for tx_result in batches {
			if let TransactionStatus::Discard(discard_status) = tx_result.status {
				tracing::info!(
					"Transaction pipe, mempool rejecting Tx:{} with status:{:?}",
					tx_result.hash,
					discard_status
				);
				core_mempool.write().unwrap().reject_transaction(
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
					"TX executed successfully.",
				);
			}
		}
		Ok(())
	}

	/// Extracts a batch of transactions from the mempool and sends it to the DA.
	pub(crate) async fn tick_mempool_sender(
		core_mempool: &RwLock<CoreMempool>,
		da_client: &(impl DaSequencerClient + 'static),
		da_batch_signer: &SignerIdentifier,
		mempool_config: &MempoolConfig,
	) -> Result<
		Option<JoinHandle<Result<movement_da_sequencer_proto::BatchWriteResponse, tonic::Status>>>,
		Error,
	> {
		let batch: Vec<Transaction> = {
			let mut core_mempool = core_mempool.write().unwrap();
			let transactions = core_mempool.get_batch_with_ranking_score(
				mempool_config.max_tx_per_batch,
				mempool_config.max_batch_size,
				true,            // allow partial batches
				BTreeMap::new(), // exclude Tx
			);

			transactions
				.into_iter()
				.map(|(transaction, ranking_score)| {
					let priority = u64::MAX - ranking_score;
					let sender = transaction.sender();
					let seq = transaction.sequence_number();
					core_mempool.commit_transaction(&sender, seq);
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
				.collect::<Result<Vec<_>, _>>()?
		};

		debug!("Get batch from mempool {}", batch.len());

		if !batch.is_empty() {
			// Build batch and submit request.
			tracing::info!("Build new batch with {} tx.", batch.len());
			let loader: LoadedSigner<Ed25519> = da_batch_signer.load().await?;

			//send the batch in a separate task to avoid to slow the loop.
			let handle = tokio::spawn({
				let mut client = da_client.clone();
				async move {
					let batch_bytes = bcs::to_bytes(&batch).expect("Serialization failed");
					let encoded =
						movement_da_sequencer_client::sign_and_encode_batch(batch_bytes, &loader)
							.await
							.unwrap();
					client.batch_write(BatchWriteRequest { data: encoded }).await
				}
			});
			Ok(Some(handle))
		} else {
			Ok(None)
		}
	}

	pub(crate) fn tick_gc(&mut self) {
		if self.last_gc.elapsed() >= GC_INTERVAL {
			debug!("Start execute mempool gc");
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
			{
				self.core_mempool.write().unwrap().gc();
			}

			self.last_gc = now;
			debug!("End execute mempool gc");
		}
	}

	// Adds a transaction to the mempool.
	async fn add_transaction_to_aptos_mempool(
		transaction: SignedTransaction,
		core_mempool: &RwLock<CoreMempool>,
		db_reader: &Arc<dyn DbReader>,
		transactions_in_flight: &Arc<RwLock<GcCounter>>,
		in_flight_limit: Option<u64>,
	) -> Result<SubmissionStatus, Error> {
		// Check whether the account is whitelisted
		// if !self.is_whitelisted(&transaction.sender())? {
		// 	return Ok((MempoolStatus::new(MempoolStatusCode::TooManyTransactions), None));
		// }

		// For now, we are going to consider a transaction in flight until it exits the mempool and is sent to the DA as is indicated by WriteBatch.
		let in_flight = {
			let transactions_in_flight = transactions_in_flight.read().unwrap();
			transactions_in_flight.get_count()
		};
		info!(
			target: "movement_timing",
			in_flight = %in_flight,
			"transactions_in_flight"
		);
		if let Some(inflight_limit) = in_flight_limit {
			if in_flight >= inflight_limit {
				warn!(
					target: "movement_timing",
					"mempool full, shedding_load"
				);
				let status = MempoolStatus::new(MempoolStatusCode::MempoolIsFull);
				return Ok((status, None));
			}
		}

		// Pre-execute Tx to validate its content.
		// Re-create the validator for each Tx because it uses a frozen version of the ledger.
		let vm_validator = VMValidator::new(Arc::clone(&db_reader));
		let tx_result = vm_validator.validate_transaction(transaction.clone())?;
		// invert the application priority with the u64 max minus the score from aptos (which is high to low)
		let ranking_score = tx_result.score();
		match tx_result.status() {
			Some(_) => {
				let ms = MempoolStatus::new(MempoolStatusCode::VmError);
				warn!(status = ?tx_result.status(), "Transaction not accepted by VM");
				return Ok((ms, tx_result.status()));
			}
			None => {
				debug!("Transaction accepted by VM");
			}
		}

		// Add the txn for future validation
		let state_view = db_reader
			.latest_state_checkpoint_view()
			.expect("Failed to get latest state checkpoint view.");
		let db_seq_num = get_account_sequence_number(&state_view, transaction.sender())?;
		info!(
			tx_sender = %transaction.sender(),
			db_seq_num = %db_seq_num,
			tx_seq_num = %transaction.sequence_number(),
		);
		let tx_hash = transaction.committed_hash();
		let status = {
			core_mempool.write().unwrap().add_txn(
				transaction,
				ranking_score,
				db_seq_num, //std::cmp::min(*db_seq_num + 1, sequence_number),
				TimelineState::NonQualified,
				true,
			)
		};

		match status.code {
			MempoolStatusCode::Accepted => {
				let now = chrono::Utc::now().timestamp_millis() as u64;
				debug!(%tx_hash, "transaction accepted by mempool");
				// increment transactions in flight
				{
					let mut transactions_in_flight = transactions_in_flight.write().unwrap();
					transactions_in_flight.increment(now, 1);
				}
			}
			_ => {
				warn!(status = ?tx_result.status(), "Transaction not accepted by mempool");
			}
		}

		// report status
		Ok((status, None))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Context, Executor, Service};
	use aptos_api::{accept_type::AcceptType, transactions::SubmitTransactionPost};
	use aptos_crypto::HashValue;
	use aptos_sdk::types::vm_status::DiscardedVMStatus;
	use aptos_storage_interface::state_view::LatestDbStateCheckpointView;
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
	use aptos_vm_validator::vm_validator;
	use futures::channel::oneshot;
	use futures::stream;
	use futures::SinkExt;
	use maptos_execution_util::config::chain::Config;
	use movement_da_sequencer_client::{ClientDaSequencerError, StreamReadBlockFromHeight};
	use movement_da_sequencer_proto::{
		BatchWriteResponse, BlockResponse, BlockV1, ReadAtHeightResponse,
	};
	use movement_types::transaction::Transaction as MvTransaction;
	use std::collections::BTreeSet;
	use std::sync::Mutex;
	use tempfile::TempDir;
	use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

	async fn setup(
	) -> (Context, TransactionPipe, TempDir, futures::channel::mpsc::Receiver<MempoolClientRequest>)
	{
		let (tx_sender, tx_receiver) = futures::channel::mpsc::channel::<MempoolClientRequest>(10);
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			unbounded_channel::<Vec<TxExecutionResult>>();
		let (executor, tempdir) =
			Executor::try_test_default(GENESIS_KEYPAIR.0.clone(), mempool_tx_exec_result_sender)
				.await
				.unwrap();
		let (context, background) =
			executor.background(mempool_commit_tx_receiver, tx_sender).unwrap();
		let transaction_pipe = background.into_transaction_pipe();
		(context, transaction_pipe, tempdir, tx_receiver)
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
		) -> Result<(StreamReadBlockFromHeight, UnboundedReceiver<()>), ClientDaSequencerError> {
			let never_ending_stream = stream::pending::<Result<BlockV1, ClientDaSequencerError>>();

			let (_alert_tx, alert_rx) = unbounded_channel();
			Ok((Box::pin(never_ending_stream), alert_rx))
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
		async fn send_state(
			&mut self,
			_signer: &LoadedSigner<Ed25519>,
			_state: movement_da_sequencer_proto::MainNodeState,
		) -> Result<movement_da_sequencer_proto::BatchWriteResponse, tonic::Status> {
			Ok(BatchWriteResponse { answer: true })
		}
		async fn read_at_height(
			&mut self,
			_height: u64,
		) -> Result<ReadAtHeightResponse, tonic::Status> {
			Ok(ReadAtHeightResponse { response: Some(BlockResponse { block_type: None }) })
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
		let (context, transaction_pipe, _tempdir, tx_receiver) = setup().await;

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone(), tx_receiver));

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
		let (context, transaction_pipe, _tempdir, tx_receiver) = setup().await;
		let user_transaction = create_signed_transaction(0, &maptos_config);

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone(), tx_receiver));

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
		let (context, transaction_pipe, _tempdir, tx_receiver) = setup().await;
		let _mempool_client_sender = context.mempool_client_sender();
		let user_transaction = create_signed_transaction(0, &maptos_config);

		let core_mempool = transaction_pipe.core_mempool();
		let db_reader = transaction_pipe.db_reader();
		let (transactions_in_flight, in_flight_limit) = transaction_pipe.transactions_in_flight();

		// Add 2 time the transaction to mempool
		let (mempool_status, _) = TransactionPipe::add_transaction_to_aptos_mempool(
			user_transaction.clone(),
			&core_mempool,
			&db_reader,
			&transactions_in_flight,
			in_flight_limit,
		)
		.await?;

		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		let (mempool_status, _) = TransactionPipe::add_transaction_to_aptos_mempool(
			user_transaction,
			&core_mempool,
			&db_reader,
			&transactions_in_flight,
			in_flight_limit,
		)
		.await?;
		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone(), tx_receiver));

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
		let (context, transaction_pipe, _tempdir, tx_receiver) = setup().await;
		let service = Service::new(&context);

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone(), tx_receiver));

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
		let (context, transaction_pipe, _tempdir, tx_receiver) = setup().await;
		let service = Service::new(&context);

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client.clone(), tx_receiver));

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
		let (_context, transaction_pipe, _tempdir, _tx_receiver) = setup().await;

		let core_mempool = transaction_pipe.core_mempool();
		let db_reader = transaction_pipe.db_reader();
		let (transactions_in_flight, in_flight_limit) = transaction_pipe.transactions_in_flight();

		// submit a transaction with a valid sequence number
		let user_transaction = create_signed_transaction(0, &maptos_config);

		// Add 2 time the transaction to mempool
		let (mempool_status, _) = TransactionPipe::add_transaction_to_aptos_mempool(
			user_transaction,
			&core_mempool,
			&db_reader,
			&transactions_in_flight,
			in_flight_limit,
		)
		.await?;

		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit a transaction with a sequence number that is too new
		let user_transaction = create_signed_transaction(34, &maptos_config);
		let (mempool_status, _) = TransactionPipe::add_transaction_to_aptos_mempool(
			user_transaction,
			&core_mempool,
			&db_reader,
			&transactions_in_flight,
			in_flight_limit,
		)
		.await?;

		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit one signed transaction with a sequence number that is too new for the vm but not for the mempool
		let user_transaction = create_signed_transaction(5, &maptos_config);
		let (mempool_status, _) = TransactionPipe::add_transaction_to_aptos_mempool(
			user_transaction,
			&core_mempool,
			&db_reader,
			&transactions_in_flight,
			in_flight_limit,
		)
		.await?;

		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		// submit a transaction with the same sequence number as the previous one
		let user_transaction = create_signed_transaction(5, &maptos_config);
		let (mempool_status, _) = TransactionPipe::add_transaction_to_aptos_mempool(
			user_transaction,
			&core_mempool,
			&db_reader,
			&transactions_in_flight,
			in_flight_limit,
		)
		.await?;

		assert_eq!(mempool_status.code, MempoolStatusCode::Accepted);

		Ok(())
	}

	#[tokio::test]
	async fn test_sequence_number_too_old() -> Result<(), anyhow::Error> {
		let (tx_sender, tx_receiver) = futures::channel::mpsc::channel::<MempoolClientRequest>(1);
		let (mempool_tx_exec_result_sender, mempool_commit_tx_receiver) =
			unbounded_channel::<Vec<TxExecutionResult>>();

		let (mut executor, _tempdir) =
			Executor::try_test_default(GENESIS_KEYPAIR.0.clone(), mempool_tx_exec_result_sender)
				.await?;
		let (context, background) = executor.background(mempool_commit_tx_receiver, tx_sender)?;
		let transaction_pipe = background.into_transaction_pipe();

		// Run the transaction pipe
		let da_client = TxPipeTestDaSequencerClient::new();
		let mempool_handle = tokio::spawn(transaction_pipe.run(da_client, tx_receiver));

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
		executor.execute_block(block)?;

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
