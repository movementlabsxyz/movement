use super::Executor;
use aptos_indexer_grpc_fullnode::{fullnode_data_service::FullnodeDataService, ServiceContext};
use aptos_protos::internal::fullnode::v1::fullnode_data_server::FullnodeDataServer;
use std::net::ToSocketAddrs;
use aptos_indexer::runtime::run_forever;

impl Executor {

    pub async fn run_indexer_grpc_service(&self) -> Result<(), anyhow::Error> {
		let indexer_context = self.context.clone();
		let indexer_config = self.node_config.indexer.clone();
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
			.map_err(|e| anyhow::anyhow!("Server error: {:?}", e))
	}

	pub async fn run_indexer_background_task(&self) -> Result<(), anyhow::Error> {
		let indexer_context = self.context.clone();
		let indexer_config = self.node_config.indexer.clone();
		run_forever(indexer_config, indexer_context.clone()).await;
		Ok(())
	}

}