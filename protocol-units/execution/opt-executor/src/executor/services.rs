use super::Executor;
use aptos_api::{
	get_api_service,
	runtime::{get_apis, Apis, root_handler},
	set_failpoints,
};
use poem::{http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server};
use tracing::info;

impl Executor {
	pub fn get_apis(&self) -> Apis {
		get_apis(self.context())
	}

	pub async fn run_service(&self) -> Result<(), anyhow::Error> {
		info!("Starting maptos-opt-executor services at: {:?}", self.listen_url);

		let size_limit = self.context.content_length_limit();
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

		Server::new(TcpListener::bind(self.listen_url.clone()))
			.run(app)
			.await
			.map_err(|e| anyhow::anyhow!("Server error: {:?}", e))?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use aptos_mempool::MempoolClientRequest;
	use aptos_types::{
		account_config, mempool_status::MempoolStatusCode, test_helpers::transaction_test_helpers,
		transaction::SignedTransaction,
	};
	use aptos_vm_genesis::GENESIS_KEYPAIR;
	use futures::channel::oneshot;
	use futures::SinkExt;
	use maptos_execution_util::config::Config;
	use aptos_mempool::core_mempool::CoreMempool;

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
		let (mut executor, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let server_executor = executor.clone();

		let handle = tokio::spawn(async move {
			server_executor.run_service().await?;
			Ok(()) as Result<(), anyhow::Error>
		});

		let user_transaction = create_signed_transaction(0, &executor.maptos_config);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		executor
			.mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		let (tx, rx) = async_channel::unbounded();
		let mut core_mempool = CoreMempool::new(&executor.node_config.clone());
		executor.tick_transaction_pipe(&mut core_mempool, tx, &mut std::time::Instant::now()).await?;

		// receive the callback
		let (status, _vm_status_code) = callback.await??;
		// dbg!(_vm_status_code);
		assert_eq!(status.code, MempoolStatusCode::Accepted);

		// receive the transaction
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, user_transaction);

		handle.abort();

		Ok(())
	}
}
