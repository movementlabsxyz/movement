use super::Context;
use aptos_db_indexer::indexer_reader::IndexerReaders;
use aptos_indexer_grpc_fullnode::runtime::bootstrap as bootstrap_indexer_grpc;
use aptos_indexer_grpc_table_info::runtime::bootstrap as bootstrap_table_info;
use aptos_types::indexer::indexer_db_reader::IndexerReader;
use std::sync::Arc;

/// Runtime handle for indexer services. This object should be kept alive
/// while services are running.
pub struct IndexerRuntime {
	_table_info_runtime: Option<tokio::runtime::Runtime>,
	_indexer_grpc: Option<tokio::runtime::Runtime>,
}

impl Context {
	// https://github.com/movementlabsxyz/aptos-core/blob/ea91067b81f9673547417bff9c70d5a2fe1b0e7b/aptos-node/src/services.rs#L40
	pub fn run_indexer_grpc_service(&self) -> Result<IndexerRuntime, anyhow::Error> {
		// If indexer grpc is not enabled, return empty runtime.
		if !self.maptos_config.chain.enable_indexer_grpc {
			return Ok(IndexerRuntime { _table_info_runtime: None, _indexer_grpc: None });
		}
		tracing::info!("Starting indexer gRPC service");
		// bootstrap table info
		let (table_info_runtime, async_indexer) = bootstrap_table_info(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.clone(),
			self.mempool_client_sender.clone(),
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap table info runtime"))?;
		// Indexer readers allow: 1. read from table info, 2 read from shard db. Shard db is not used thus None.
		let indexer_readers = IndexerReaders::new(Some(async_indexer), None);
		let indexer_reader: Option<Arc<dyn IndexerReader>> = indexer_readers.map(|readers| {
			let trait_object: Arc<dyn IndexerReader> = Arc::new(readers);
			trait_object
		});

		// Bootstrap indexer grpc, aka, transaction stream service.
		// TSS requires the table info service and internal indexer db.
		let indexer_grpc = bootstrap_indexer_grpc(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.reader.clone(),
			self.mempool_client_sender.clone(),
			indexer_reader,
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap indexer grpc runtime"))?;

		Ok(IndexerRuntime {
			_table_info_runtime: Some(table_info_runtime),
			_indexer_grpc: Some(indexer_grpc),
		})
	}
}
