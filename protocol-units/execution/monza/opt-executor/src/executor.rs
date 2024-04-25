use aptos_db::AptosDB;
use aptos_executor_types::{state_checkpoint_output::StateCheckpointOutput, BlockExecutorTrait};
use aptos_mempool::{
	MempoolClientRequest, MempoolClientSender,
};
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
	block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableBlock}, chain_id::ChainId, transaction::{
		SignedTransaction, Transaction, WriteSetPayload
	}, validator_signer::ValidatorSigner
};
use aptos_vm::AptosVM;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use aptos_config::config::NodeConfig;
use aptos_executor::{
	block_executor::BlockExecutor,
	db_bootstrapper::{generate_waypoint, maybe_bootstrap},
};
use aptos_api::{get_api_service, runtime::{get_apis, Apis}, Context};
use futures::channel::mpsc as futures_mpsc;
use poem::{listener::TcpListener, Route, Server};
use aptos_sdk::types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_mempool::SubmissionStatus;
use futures::StreamExt;
/*use aptos_faucet_core::{
	bypasser::{Bypasser, BypasserConfig},
    checkers::{CaptchaManager, Checker, CheckerConfig, CheckerTrait},
    endpoints::{
        build_openapi_service, convert_error, mint, BasicApi, CaptchaApi, FundApi,
        FundApiComponents,
    },
    funder::{ApiConnectionConfig, FunderConfig, MintFunderConfig, TransactionSubmissionConfig},
    middleware::middleware_log,
};
use tokio::sync::Semaphore;*/

/// The `Executor` is responsible for executing blocks and managing the state of the execution
/// against the `AptosVM`.
#[derive(Clone)]
pub struct Executor {
	/// The executing type.
	pub block_executor: Arc<RwLock<BlockExecutor<AptosVM>>>,
	/// The access to db.
	pub db: Arc<RwLock<DbReaderWriter>>,
	/// The signer of the executor's transactions.
	pub signer: ValidatorSigner,
	/// The sender for the mempool client.
	pub mempool_client_sender: MempoolClientSender,
	/// The receiver for the mempool client.
	pub mempool_client_receiver: Arc<RwLock<futures_mpsc::Receiver<MempoolClientRequest>>>,
	/// The configuration of the node.
	pub node_config: NodeConfig,
	/// The chain id of the node.
	pub chain_id: ChainId,
	/// Context 
	pub context : Arc<Context>,
}

impl Executor {

	const DB_PATH_ENV_VAR: &'static str = "DB_DIR";

	/// Create a new `Executor` instance.
	pub fn new(
		db_dir : PathBuf,
		block_executor: BlockExecutor<AptosVM>,
		signer: ValidatorSigner,
		mempool_client_sender: MempoolClientSender,
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		node_config: NodeConfig,
		chain_id: ChainId,
	) -> Self {

		let (_aptos_db, reader_writer) = DbReaderWriter::wrap(AptosDB::new_for_test(&db_dir));
		let reader = reader_writer.reader.clone();
		Self {
			block_executor: Arc::new(RwLock::new(block_executor)),
			db: Arc::new(RwLock::new(reader_writer)),
			signer,
			mempool_client_sender : mempool_client_sender.clone(),
			node_config : node_config.clone(),
			mempool_client_receiver : Arc::new(RwLock::new(mempool_client_receiver)),
			chain_id : chain_id.clone(),
			context : Arc::new(Context::new(
				chain_id,
				reader,
				mempool_client_sender,
				node_config ,
				None
			))
		}
	}

	pub fn bootstrap_empty_db(db_dir : PathBuf) -> Result<DbReaderWriter, anyhow::Error> {
		let genesis = aptos_vm_genesis::test_genesis_change_set_and_validators(Some(1));
		let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis.0));
		let db_rw = DbReaderWriter::new(AptosDB::new_for_test(&db_dir));
		assert!(db_rw.reader.get_latest_ledger_info_option()?.is_none());

		// Bootstrap empty DB.
		let waypoint =
			generate_waypoint::<AptosVM>(&db_rw, &genesis_txn).expect("Should not fail.");
		maybe_bootstrap::<AptosVM>(&db_rw, &genesis_txn, waypoint)?;
		assert!(db_rw.reader.get_latest_ledger_info_option()?.is_some());

		Ok(db_rw)
	}

	pub fn bootstrap(
		db_dir : PathBuf,
		signer: ValidatorSigner,
		mempool_client_sender: MempoolClientSender,
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		node_config: NodeConfig,
		chain_id: ChainId,
	) -> Result<Self, anyhow::Error> {

		let db_rw = Self::bootstrap_empty_db(db_dir)?;
		let reader = db_rw.reader.clone();

		Ok(Self {
			block_executor: Arc::new(RwLock::new(BlockExecutor::new(db_rw.clone()))),
			db: Arc::new(RwLock::new(db_rw)),
			signer,
			mempool_client_sender : mempool_client_sender.clone(),
			mempool_client_receiver : Arc::new(RwLock::new(mempool_client_receiver)),
			node_config : node_config.clone(),
			chain_id,
			context : Arc::new(Context::new(
				chain_id,
				reader,
				mempool_client_sender,
				node_config,
				None
			))
		})

	}

	pub fn try_from_env() -> Result<Self, anyhow::Error> {

		// read the db dir from env or use a tempfile
		let db_dir = match std::env::var(Self::DB_PATH_ENV_VAR) {
			Ok(dir) => PathBuf::from(dir),
			Err(_) => {
				let temp_dir = tempfile::tempdir()?;
				temp_dir.path().to_path_buf()
			}
		};

		// use the default signer, block executor, and mempool
		let signer = ValidatorSigner::random(None);
		let (mempool_client_sender, mempool_client_receiver) = futures_mpsc::channel::<MempoolClientRequest>(10);
		let node_config = NodeConfig::default();
		let chain_id = ChainId::new(10);

		Self::bootstrap(
			db_dir,
			signer,
			mempool_client_sender,
			mempool_client_receiver,
			node_config,
			chain_id,
		)

	}

	/// Execute a block which gets committed to the state.
	/// `ExecutorState` must be set to `Commit` before calling this method.
	pub async fn execute_block(
		&self,
		block: ExecutableBlock,
	) -> Result<StateCheckpointOutput, anyhow::Error> {

		let parent_block_id = {
			let block_executor = self.block_executor.read().await;
			block_executor.committed_block_id()
		};

	
		let state_checkpoint = {
			let block_executor = self.block_executor.write().await;
			block_executor.execute_and_state_checkpoint(
				block,
				parent_block_id,
				BlockExecutorConfigFromOnchain::new_no_block_limit(),
			)?
		};

		Ok(state_checkpoint)
	}

	pub async fn try_get_context(&self) -> Result<Arc<Context>, anyhow::Error> {
		Ok(self.context.clone())
	}

	pub async fn try_get_apis(&self) -> Result<Apis, anyhow::Error> {
		let context = self.try_get_context().await?;
		Ok(get_apis(context))
	}

	pub async fn run_service(&self) -> Result<(), anyhow::Error> {

		let context = self.try_get_context().await?;
		let api_service = get_api_service(context).server("http://127.0.0.1:3000");

		/*let basic_api = BasicApi {
			concurrent_requests_semaphore : None,

		};*/

		let ui = api_service.swagger_ui();
	
		let app = Route::new()
			.nest("/v1", api_service)
			.nest("/spec", ui);
		Server::new(TcpListener::bind("127.0.0.1:3000"))
			.run(app)
			.await.map_err(
				|e| anyhow::anyhow!("Server error: {:?}", e)
			)?;

		Ok(())
	}

	/// Pipes a batch of transactions from the mempool to the transaction channel.
	/// todo: it may be wise to move the batching logic up a level to the consuming structs.
	pub async fn tick_transaction_pipe(
		&self, 
		transaction_channel : async_channel::Sender<SignedTransaction>
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
									transaction_channel.send(transaction).await?;
									let ms = MempoolStatus::new(MempoolStatusCode::Accepted);
									let status: SubmissionStatus = (ms, None);
									callback.send(Ok(status)).map_err(
										|e| anyhow::anyhow!("Error sending callback: {:?}", e)
									)?;
								},
								MempoolClientRequest::GetTransactionByHash(_, _) => {},
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

}

#[cfg(test)]
mod tests {

	use super::*;
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		HashValue, PrivateKey, Uniform,
	};
	use aptos_types::{
		account_address::AccountAddress,
		block_executor::partitioner::ExecutableTransactions,
		chain_id::ChainId,
		transaction::{
			signature_verified_transaction::SignatureVerifiedTransaction, RawTransaction, Script,
			SignedTransaction, Transaction, TransactionPayload
		}
	};
	use aptos_api::{
		accept_type::AcceptType,
		transactions::SubmitTransactionPost
	};
	use futures::SinkExt;
	use futures::channel::oneshot;

	fn create_signed_transaction(gas_unit_price: u64) -> SignedTransaction {
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
			ChainId::new(10), // This is the value used in aptos testing code.
		);
		SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
	}


	#[tokio::test]
	async fn test_execute_block() -> Result<(), anyhow::Error> {
		let mut executor = Executor::try_from_env()?;
		let block_id = HashValue::random();
		let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
			create_signed_transaction(0),
		));
		let txs = ExecutableTransactions::Unsharded(vec![tx]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(block).await?;
		Ok(())
	}

	#[tokio::test]
	async fn test_pipe_mempool() -> Result<(), anyhow::Error> {

		// header
		let mut executor = Executor::try_from_env()?;
		let user_transaction = create_signed_transaction(0);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		executor.mempool_client_sender.send(MempoolClientRequest::SubmitTransaction(
			user_transaction.clone(),
			req_sender
		)).await?;

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
	async fn test_pipe_mempool_while_server_running() -> Result<(), anyhow::Error> {
		
		let mut executor = Executor::try_from_env()?;
		let server_executor = executor.clone();

		let handle = tokio::spawn(async move {
			server_executor.run_service().await?;
			Ok(()) as Result<(), anyhow::Error> 
		});

		let user_transaction = create_signed_transaction(0);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		executor.mempool_client_sender.send(MempoolClientRequest::SubmitTransaction(
			user_transaction.clone(),
			req_sender
		)).await?;

		// tick the transaction pipe
		let (tx, rx) = async_channel::unbounded();
		executor.tick_transaction_pipe(tx).await?;

		// receive the callback
		callback.await??;
		
		// receive the transaction
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, user_transaction);

		handle.abort();

		Ok(())
	}


	#[tokio::test]
	async fn test_pipe_mempool_from_api() -> Result<(), anyhow::Error> {

		let executor = Executor::try_from_env()?;
		let mempool_executor = executor.clone();
		let user_transaction = create_signed_transaction(0);
		let comparison_user_transaction = user_transaction.clone();
		let bcs_user_transaction = bcs::to_bytes(&user_transaction)?;

		let (tx, rx) = async_channel::unbounded();
		let mempool_handle = tokio::spawn(async move {
			loop {
				mempool_executor.tick_transaction_pipe(tx.clone()).await?;
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			};
			Ok(()) as Result<(), anyhow::Error>
		});

		let request = SubmitTransactionPost::Bcs(
			aptos_api::bcs_payload::Bcs(bcs_user_transaction)
		);
		let api = executor.try_get_apis().await?;
		api.transactions.submit_transaction(AcceptType::Bcs, request).await?;

		mempool_handle.abort();
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, comparison_user_transaction);
	
		Ok(())
	}

}
