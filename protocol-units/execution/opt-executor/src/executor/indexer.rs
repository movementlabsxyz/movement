use super::Executor;
use aptos_indexer_grpc_fullnode::{fullnode_data_service::FullnodeDataService, ServiceContext};
use aptos_protos::internal::fullnode::v1::fullnode_data_server::FullnodeDataServer;
use std::net::ToSocketAddrs;
use aptos_indexer::runtime::run_forever;
use aptos_indexer_grpc_table_info::runtime::bootstrap as bootstrap_table_info;
use aptos_indexer_grpc_fullnode::runtime::bootstrap as bootstrap_indexer_grpc;
use aptos_indexer::runtime::bootstrap as bootstrap_indexer_stream;

impl Executor {

	// https://github.com/movementlabsxyz/aptos-core/blob/ea91067b81f9673547417bff9c70d5a2fe1b0e7b/aptos-node/src/services.rs#L40
    pub async fn run_indexer_grpc_service(&self) -> Result<(), anyhow::Error> {

		// bootstrap table info
		let (runtime, _async_indexer_v2) = bootstrap_table_info(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.clone(),
			self.mempool_client_sender.clone(),
		).ok_or(
			anyhow::anyhow!("Failed to bootstrap table info runtime"),
		)?;

		// bootstrap indexer grpc
		let indexer_grpc = bootstrap_indexer_grpc(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.reader.clone(),
			self.mempool_client_sender.clone(),
			None
		).ok_or(
			anyhow::anyhow!("Failed to bootstrap indexer grpc runtime"),
		)?;

		// bootstrap indexer stream
		let indexer_stream = bootstrap_indexer_stream(
			&self.node_config,
			self.maptos_config.chain.maptos_chain_id.clone(),
			self.db.reader.clone(),
			self.mempool_client_sender.clone(),
		).ok_or(
			anyhow::anyhow!("Failed to bootstrap indexer stream runtime"),
		)?;
	
		/*let indexer_context = self.context.clone();
		let server = FullnodeDataService {
			service_context: ServiceContext {
				context: indexer_context.clone(),
				processor_task_count: 4,
				processor_batch_size: 4,
				output_batch_size: 4,
			},
		};

		tonic::transport::Server::builder()
			.add_service(FullnodeDataServer::new(server))
			.serve(String::from("0.0.0.0:8090").to_socket_addrs().unwrap().next().unwrap())
			.await
			.map_err(|e| anyhow::anyhow!("Server error: {:?}", e))*/

		// sleep forever
		tokio::time::sleep(tokio::time::Duration::from_secs(100000)).await;
		Ok(())
	}

	pub async fn run_indexer_background_task(&self) -> Result<(), anyhow::Error> {
		/*let indexer_context = self.context.clone();
		let indexer_config = self.node_config.indexer.clone();
		run_forever(indexer_config, indexer_context.clone()).await;*/
		Ok(())
	}

}