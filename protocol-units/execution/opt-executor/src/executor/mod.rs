//! Implementation is split over multiple files to make the code more manageable.
pub mod execution;
pub mod initialization;
pub mod services;
pub mod transaction_pipe;
use anyhow::Context as _;
use aptos_config::config::NodeConfig;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_mempool::{core_mempool::CoreMempool, MempoolClientRequest, MempoolClientSender};
use aptos_storage_interface::DbReaderWriter;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use futures::channel::mpsc as futures_mpsc;
use std::sync::Arc;
use tokio::sync::RwLock;
use aptos_api::context::Context;

/// The `Executor` is responsible for executing blocks and managing the state of the execution
/// against the `AptosVM`.
#[derive(Clone)]
pub struct Executor {
	/// The executing type.
	pub block_executor: Arc<RwLock<BlockExecutor<AptosVM>>>,
	/// The access to db.
	pub db: DbReaderWriter,
	/// The signer of the executor's transactions.
	pub signer: ValidatorSigner,
	/// The core mempool (used for the api to query the mempool).
	pub core_mempool: Arc<RwLock<CoreMempool>>,
	/// The sender for the mempool client.
	pub mempool_client_sender: MempoolClientSender,
	/// The receiver for the mempool client.
	pub mempool_client_receiver: Arc<RwLock<futures_mpsc::Receiver<MempoolClientRequest>>>,
	/// The configuration of the node.
	pub node_config: NodeConfig,
	/// Context
	pub context: Arc<Context>,
	/// URL for the API endpoint
	pub listen_url: String,
	/// Maptos config
	pub maptos_config: maptos_execution_util::config::Config,
}

impl Executor {
	/// Create a new `Executor` instance.
	pub fn try_new(
		block_executor: BlockExecutor<AptosVM>,
		signer: ValidatorSigner,
		mempool_client_sender: MempoolClientSender,
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		node_config: NodeConfig,
		maptos_config: maptos_execution_util::config::Config,
	) -> Result<Self, anyhow::Error> {
		let (_aptos_db, reader_writer) =
			DbReaderWriter::wrap(AptosDB::new_for_test(&maptos_config.chain.maptos_db_path.clone().context("No db path provided.")?));
		let core_mempool = Arc::new(RwLock::new(CoreMempool::new(&node_config)));
		let reader = reader_writer.reader.clone();
		Ok(Self {
			block_executor: Arc::new(RwLock::new(block_executor)),
			db: reader_writer,
			signer,
			core_mempool,
			mempool_client_sender: mempool_client_sender.clone(),
			node_config: node_config.clone(),
			mempool_client_receiver: Arc::new(RwLock::new(mempool_client_receiver)),
			context: Arc::new(Context::new(
				maptos_config.chain.maptos_chain_id.clone(),
				reader,
				mempool_client_sender,
				node_config,
				None,
			)),
			listen_url: format!(
				"{}:{}",
				maptos_config.chain.maptos_rest_listen_hostname,
				maptos_config.chain.maptos_rest_listen_port
			),
			maptos_config,
		})
	}
}
