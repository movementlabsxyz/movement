use crate::Context;

use aptos_api::{
	get_api_service,
	runtime::{get_apis, root_handler, Apis},
	set_failpoints,
};
use aptos_storage_interface::DbReaderWriter;

use poem::{http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server};
use tracing::info;

use std::sync::Arc;

pub struct Service {
	// API context
	context: Arc<aptos_api::Context>,
	// URL for the API endpoint
	listen_url: String,
}

impl Service {
	pub fn new(cx: &Context) -> Self {
		let Context {
			chain_config,
			db: DbReaderWriter { reader, .. },
			mempool_client_sender,
			node_config,
		} = cx;
		let context = Arc::new(aptos_api::Context::new(
			chain_config.maptos_chain_id.clone(),
			reader.clone(),
			mempool_client_sender.clone(),
			node_config.clone(),
			None,
		));
		let listen_url = format!(
			"{}:{}",
			chain_config.maptos_rest_listen_hostname, chain_config.maptos_rest_listen_port
		);
		Service { context, listen_url }
	}

	fn context(&self) -> Arc<aptos_api::Context> {
		Arc::clone(&self.context)
	}

	pub fn get_apis(&self) -> Apis {
		get_apis(self.context())
	}

	pub async fn run(&self) -> Result<(), anyhow::Error> {
		info!("Starting maptos-opt-executor services at: {:?}", self.listen_url);

		let api_service =
			get_api_service(self.context()).server(format!("http://{:?}", self.listen_url));

		let spec_json = api_service.spec_endpoint();
		let spec_yaml = api_service.spec_endpoint_yaml();

		let ui = api_service.swagger_ui();

		let cors = Cors::new()
			.allow_methods(vec![Method::GET, Method::POST])
			.allow_credentials(true);
		let app = Route::new()
			.at("/", poem::get(root_handler))
			.nest("/v1", api_service)
			.nest("/spec", ui)
			.at("/spec.json", poem::get(spec_json))
			.at("/spec.yaml", poem::get(spec_yaml))
			.at(
				"/set_failpoint",
				poem::get(set_failpoints::set_failpoint_poem).data(self.context()),
			)
			.with(cors);

		Server::new(TcpListener::bind(&self.listen_url))
			.run(app)
			.await
			.map_err(|e| anyhow::anyhow!("Server error: {:?}", e))?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Executor;
	use aptos_mempool::MempoolClientRequest;
	use aptos_types::{
		account_config, mempool_status::MempoolStatusCode, test_helpers::transaction_test_helpers,
		transaction::SignedTransaction,
	};
	use aptos_vm_genesis::GENESIS_KEYPAIR;
	use futures::channel::oneshot;
	use futures::SinkExt;
	use maptos_execution_util::config::Config;
	use tokio::sync::mpsc;

	fn create_signed_transaction(
		sequence_number: u64,
		maptos_config: &Config,
	) -> SignedTransaction {
		let address = account_config::aptos_test_root_address();
		transaction_test_helpers::get_test_txn_with_chain_id(
			address,
			sequence_number,
			&GENESIS_KEYPAIR.0,
			GENESIS_KEYPAIR.1.clone(),
			maptos_config.chain.maptos_chain_id.clone(), // This is the value used in aptos testing code.
		)
	}

	#[tokio::test]
	async fn test_pipe_mempool_while_server_running() -> Result<(), anyhow::Error> {
		let (executor, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let (tx_sender, mut tx_receiver) = mpsc::channel(16);
		let (mut context, mut transaction_pipe) = executor.background(tx_sender);
		let service = Service::new(&context);
		let handle = tokio::spawn(async move { service.run().await });

		let user_transaction = create_signed_transaction(0, &executor.maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		context
			.mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		transaction_pipe.tick().await?;

		// receive the callback
		let (status, _vm_status_code) = callback.await??;
		// dbg!(_vm_status_code);
		assert_eq!(status.code, MempoolStatusCode::Accepted);

		// receive the transaction
		let received_transaction = tx_receiver.recv().await.unwrap();
		assert_eq!(received_transaction, user_transaction);

		handle.abort();

		Ok(())
	}
}
