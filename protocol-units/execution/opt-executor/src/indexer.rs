use super::Context;

use aptos_indexer::runtime::bootstrap as bootstrap_indexer_stream;
use aptos_indexer_grpc_fullnode::runtime::bootstrap as bootstrap_indexer_grpc;
use aptos_indexer_grpc_table_info::runtime::bootstrap as bootstrap_table_info;

use tokio::runtime::Runtime;

/// Runtime handle for indexer services. This object should be kept alive
/// while services are running.
#[allow(dead_code)] // we only keep the runtimes around to drop them
pub struct IndexerRuntime {
	table_info_runtime: Runtime,
	indexer_grpc: Runtime,
	indexer_stream: Runtime,
}

impl Context {
	// https://github.com/movementlabsxyz/aptos-core/blob/ea91067b81f9673547417bff9c70d5a2fe1b0e7b/aptos-node/src/services.rs#L40
	pub fn run_indexer_grpc_service(&self) -> Result<IndexerRuntime, anyhow::Error> {
		tracing::info!("Starting indexer gRPC with node config {:?}", self.node_config);

		// bootstrap table info
		let (table_info_runtime, _async_indexer) = bootstrap_table_info(
			&self.node_config,
			self.chain_config.maptos_chain_id.clone(),
			self.db.clone(),
			self.mempool_client_sender.clone(),
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap table info runtime"))?;

		// bootstrap indexer grpc
		// this one actually serves the gRPC service
		let indexer_grpc = bootstrap_indexer_grpc(
			&self.node_config,
			self.chain_config.maptos_chain_id.clone(),
			self.db.reader.clone(),
			self.mempool_client_sender.clone(),
			None,
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap indexer grpc runtime"))?;

		// bootstrap indexer stream
		let indexer_stream = bootstrap_indexer_stream(
			&self.node_config,
			self.chain_config.maptos_chain_id.clone(),
			self.db.reader.clone(),
			self.mempool_client_sender.clone(),
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap indexer stream runtime"))??;

		Ok(IndexerRuntime { table_info_runtime, indexer_grpc, indexer_stream })
	}
}
