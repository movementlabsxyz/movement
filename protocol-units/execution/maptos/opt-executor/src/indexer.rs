use super::Context;
use aptos_indexer_grpc_fullnode::runtime::bootstrap as bootstrap_indexer_grpc;
use aptos_indexer_grpc_table_info::runtime::bootstrap as bootstrap_table_info;

use tokio::runtime::Runtime;

/// Runtime handle for indexer services. This object should be kept alive
/// while services are running.
pub struct IndexerRuntime {
	// We only keep the runtimes around to drop them
	_table_info_runtime: Runtime,
	_indexer_grpc: Runtime,
	//	_indexer_stream: Runtime,
}

impl Context {
	// https://github.com/movementlabsxyz/aptos-core/blob/ea91067b81f9673547417bff9c70d5a2fe1b0e7b/aptos-node/src/services.rs#L40
	pub fn run_indexer_grpc_service(&self) -> Result<IndexerRuntime, anyhow::Error> {
		tracing::info!("Starting indexer gRPC service");

		// bootstrap table info
		let (_table_info_runtime, _async_indexer) = bootstrap_table_info(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.clone(),
			self.mempool_client_sender.clone(),
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap table info runtime"))?;

		// Bootstrap indexer grpc, aka, transaction stream service.
		// TSS requires the table info service and internal indexer db.
		// TODO: add mempool client receiver to the bootstrap function.
		let _indexer_grpc = bootstrap_indexer_grpc(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.reader.clone(),
			self.mempool_client_sender.clone(),
			None,
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap indexer grpc runtime"))?;

		Ok(IndexerRuntime { _table_info_runtime, _indexer_grpc }) //, _indexer_stream
	}
}
