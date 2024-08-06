use aptos_config::config::NodeConfig;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::{DbReader, DbReaderWriter};
use maptos_execution_util::config::chain::Config as ChainConfig;

use std::sync::Arc;

/// Infrastructure shared by services using the storage and the mempool.
pub struct Context {
	pub(crate) db: DbReaderWriter,
	pub(crate) mempool_client_sender: MempoolClientSender,
	pub(crate) chain_config: ChainConfig,
	pub(crate) node_config: NodeConfig,
}

impl Context {
	pub(crate) fn new(
		db: DbReaderWriter,
		mempool_client_sender: MempoolClientSender,
		chain_config: ChainConfig,
		node_config: NodeConfig,
	) -> Self {
		Context { db, mempool_client_sender, chain_config, node_config }
	}

	/// Returns a reference on the data store reader.
	pub fn db_reader(&self) -> Arc<dyn DbReader> {
		Arc::clone(&self.db.reader)
	}

	/// Returns a clone of the mempool client channel's sender.
	pub fn mempool_client_sender(&self) -> MempoolClientSender {
		self.mempool_client_sender.clone()
	}
}
