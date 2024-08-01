use super::Executor;
use crate::{bootstrap, Context, TransactionPipe};

use aptos_config::config::NodeConfig;
#[cfg(test)]
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::PrivateKey;
use aptos_executor::block_executor::BlockExecutor;
use aptos_mempool::MempoolClientRequest;
use aptos_types::transaction::SignedTransaction;
use maptos_execution_util::config::Config;

use anyhow::Context as _;
use futures::channel::mpsc as futures_mpsc;
use tokio::sync::mpsc;

#[cfg(test)]
use tempfile::TempDir;

use std::net::ToSocketAddrs;
use std::sync::{atomic::AtomicU64, Arc};

// executor channel size
const EXECUTOR_CHANNEL_SIZE: usize = 2_usize.pow(16);

impl Executor {
	pub fn bootstrap(maptos_config: &Config) -> Result<Self, anyhow::Error> {
		let (db, signer) = bootstrap::maybe_bootstrap_empty_db(
			maptos_config.chain.maptos_db_path.as_ref().context("No db path provided.")?,
			maptos_config.chain.maptos_chain_id.clone(),
			&maptos_config.chain.maptos_private_key.public_key(),
		)?;
		Ok(Self {
			block_executor: Arc::new(BlockExecutor::new(db.clone())),
			signer,
			transactions_in_flight: Arc::new(AtomicU64::new(0)),
		})
	}

	pub fn try_from_config(maptos_config: &Config) -> Result<Self, anyhow::Error> {
		Self::bootstrap(maptos_config)
	}

	#[cfg(test)]
	pub fn try_test_default(
		private_key: Ed25519PrivateKey,
	) -> Result<(Self, Config, TempDir), anyhow::Error> {
		let tempdir = tempfile::tempdir()?;

		let mut maptos_config = Config::default();
		maptos_config.chain.maptos_private_key = private_key;

		// replace the db path with the temporary directory
		maptos_config.chain.maptos_db_path.replace(tempdir.path().to_path_buf());
		let executor = Self::try_from_config(&maptos_config)?;
		Ok((executor, maptos_config, tempdir))
	}

	/// Creates instance of `Context` and the background `TransactionPipe`
	/// task to process transactions.
	pub fn background(
		&self,
		transaction_sender: mpsc::Sender<SignedTransaction>,
		maptos_config: &Config,
	) -> anyhow::Result<(Context, TransactionPipe)> {
		let mut node_config = NodeConfig::default();

		node_config.indexer.enabled = true;
		// indexer config
		node_config.indexer.postgres_uri =
			Some(maptos_config.indexer_processor.postgres_connection_string.clone());
		node_config.indexer.processor = Some("default_processor".to_string());
		node_config.indexer.check_chain_id = Some(false);
		node_config.indexer.skip_migrations = Some(false);
		node_config.indexer.fetch_tasks = Some(4);
		node_config.indexer.processor_tasks = Some(4);
		node_config.indexer.emit_every = Some(4);
		node_config.indexer.batch_size = Some(8);
		node_config.indexer.gap_lookback_versions = Some(4);

		node_config.indexer_grpc.enabled = true;

		// indexer_grpc config
		node_config.indexer_grpc.processor_batch_size = 4;
		node_config.indexer_grpc.processor_task_count = 4;
		node_config.indexer_grpc.output_batch_size = 4;
		node_config.indexer_grpc.address = (
			maptos_config.indexer.maptos_indexer_grpc_listen_hostname.as_str(),
			maptos_config.indexer.maptos_indexer_grpc_listen_port,
		)
			.to_socket_addrs()?
			.next()
			.context("failed to resolve the value of maptos_indexer_grpc_listen_hostname")?;
		node_config.indexer_grpc.use_data_service_interface = true;

		// indexer table info config
		node_config.indexer_table_info.enabled = true;
		node_config.storage.dir = "./.movement/maptos-storage".to_string().into();
		node_config.storage.set_data_dir(node_config.storage.dir.clone());

		// use the default signer, block executor, and mempool
		let (mempool_client_sender, mempool_client_receiver) =
			futures_mpsc::channel::<MempoolClientRequest>(2 ^ 16); // allow 2^16 transactions before apply backpressure given theoretical maximum TPS of 170k
		let transaction_pipe = TransactionPipe::new(
			mempool_client_receiver,
			transaction_sender,
			self.db().reader.clone(),
			&node_config,
			Arc::clone(&self.transactions_in_flight),
		);
		let cx = Context::new(
			self.db().clone(),
			mempool_client_sender,
			maptos_config.clone(),
			node_config,
		);
		Ok((cx, transaction_pipe))
	}
}
