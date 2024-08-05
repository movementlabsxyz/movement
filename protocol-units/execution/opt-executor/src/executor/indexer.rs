use super::Executor;
/*use aptos_indexer_grpc_fullnode::{fullnode_data_service::FullnodeDataService, ServiceContext};
use aptos_protos::internal::fullnode::v1::fullnode_data_server::FullnodeDataServer;
use std::net::ToSocketAddrs;
use aptos_indexer::runtime::run_forever;*/
use aptos_indexer::runtime::bootstrap as bootstrap_indexer_stream;
use aptos_indexer_grpc_fullnode::runtime::bootstrap as bootstrap_indexer_grpc;
use aptos_indexer_grpc_table_info::runtime::bootstrap as bootstrap_table_info;

impl Executor {
	// https://github.com/movementlabsxyz/aptos-core/blob/ea91067b81f9673547417bff9c70d5a2fe1b0e7b/aptos-node/src/services.rs#L40
	pub async fn run_indexer_grpc_service(&self) -> Result<(), anyhow::Error> {
		tracing::info!("Starting indexer gRPC with node config {:?}", self.node_config);

		// bootstrap table info
		let (_table_info_runtime, _async_indexer_v2) = bootstrap_table_info(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.clone(),
			self.mempool_client_sender.clone(),
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap table info runtime"))?;

		// bootstrap indexer grpc
		// this one actually serves the gRPC service
		let _indexer_grpc = bootstrap_indexer_grpc(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.reader.clone(),
			self.mempool_client_sender.clone(),
			None,
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap indexer grpc runtime"))?;

		// bootstrap indexer stream
		let _indexer_stream = bootstrap_indexer_stream(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.reader.clone(),
			self.mempool_client_sender.clone(),
		)
		.ok_or(anyhow::anyhow!("Failed to bootstrap indexer stream runtime"))?;

		// sleep forever
		Ok(futures::future::pending::<()>().await)
	}
}
