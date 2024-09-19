use crate::Context;

use aptos_api::{
	get_api_service,
	runtime::{get_apis, root_handler, Apis},
	set_failpoints,
};
use aptos_storage_interface::DbReaderWriter;

use futures::prelude::*;
use poem::{
	get, handler, http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server,
};

use poem::{
	get, handler,
	middleware::Tracing,
	web::{Data, Path},
	EndpointExt, IntoResponse, Response, Route, Server,
};

use tracing::info;

use std::future::Future;
use std::sync::Arc;

#[derive(Clone)]
pub struct Service {
	// API context
	context: Arc<aptos_api::Context>,
	// URL for the API endpoint
	listen_url: String,
}

impl Service {
	pub fn new(cx: &Context) -> Self {
		let Context {
			db: DbReaderWriter { reader, .. },
			mempool_client_sender,
			maptos_config,
			node_config,
		} = cx;
		let context = Arc::new(aptos_api::Context::new(
			maptos_config.chain.maptos_chain_id.clone(),
			reader.clone(),
			mempool_client_sender.clone(),
			node_config.clone(),
			None,
		));
		let listen_url = format!(
			"{}:{}",
			maptos_config.chain.maptos_rest_listen_hostname,
			maptos_config.chain.maptos_rest_listen_port
		);
		Service { context, listen_url }
	}

	pub fn api_context(&self) -> Arc<aptos_api::Context> {
		Arc::clone(&self.context)
	}

	pub fn get_apis(&self) -> Apis {
		get_apis(self.api_context())
	}

	pub fn run(&self) -> impl Future<Output = Result<(), anyhow::Error>> + Send {
		info!("Starting maptos-opt-executor services at: {:?}", self.listen_url);

		let api_service =
			get_api_service(self.api_context()).server(format!("http://{:?}", self.listen_url));

		let spec_json = api_service.spec_endpoint();
		let spec_yaml = api_service.spec_endpoint_yaml();

		let ui = api_service.swagger_ui();

		let cors = Cors::new()
			.allow_methods(vec![Method::GET, Method::POST])
			.allow_credentials(true);
		let listener = TcpListener::bind(self.listen_url.clone());
		let app = Route::new()
			.at("/", poem::get(root_handler))
			.nest("/v1", api_service)
			.nest("/spec", ui)
			.at("/v1/state-root-hash/:blockheight", get(state_root_hash))
			.at("/spec.json", poem::get(spec_json))
			.at("/spec.yaml", poem::get(spec_yaml))
			.at(
				"/set_failpoint",
				poem::get(set_failpoints::set_failpoint_poem).data(self.api_context()),
			)
			.data(self.context.clone())
			.with(cors);

		Server::new(listener)
			.run(app)
			.map_err(|e| anyhow::anyhow!("Server error: {:?}", e))
	}
}

#[handler]
pub async fn state_root_hash(
	Path(blockheight): Path<u64>,
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	let latest_ledger_info = context.db.get_latest_ledger_info()?;
	let (_, end_version, _) = context.db.get_block_info_by_height(blockheight)?;
	tracing::info!("end_version: {}", end_version);
	let txn_with_proof = context.db.get_transaction_by_version(
		end_version,
		latest_ledger_info.ledger_info().version(),
		false,
	)?;
	tracing::info!("txn_with_proof: {:?}", txn_with_proof);
	let state_root_hash = txn_with_proof
		.proof
		.transaction_info
		.state_checkpoint_hash()
		.ok_or_else(|| anyhow::anyhow!("No state root hash found"))?;
	Ok(state_root_hash.to_string().into_response())
}

#[handler]
pub async fn get_current_commitment(
	context: Data<&Arc<Context>>,
) -> Result<Response, anyhow::Error> {
	let latest_ledger_info = context.db.get_latest_ledger_info()?;
	let version = latest_ledger_info.ledger_info().version();
	let state_proof = context.db.get_state_proof(version)?;
	let commitment = movement_types::Commitment::digest_state_proof(&state_proof);
	Ok(hex::encode(&commitment.0).into_response())
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
	use maptos_execution_util::config::chain::Config;
	use tokio::sync::mpsc;

	fn create_signed_transaction(sequence_number: u64, chain_config: &Config) -> SignedTransaction {
		let address = account_config::aptos_test_root_address();
		transaction_test_helpers::get_test_txn_with_chain_id(
			address,
			sequence_number,
			&GENESIS_KEYPAIR.0,
			GENESIS_KEYPAIR.1.clone(),
			chain_config.maptos_chain_id.clone(), // This is the value used in aptos testing code.
		)
	}

	#[tokio::test]
	async fn test_pipe_mempool_while_server_running() -> Result<(), anyhow::Error> {
		let (tx_sender, mut tx_receiver) = mpsc::channel(16);
		let (executor, config, _tempdir) = Executor::try_test_default(GENESIS_KEYPAIR.0.clone())?;
		let (context, mut transaction_pipe, _indexer_runtime) =
			executor.background(tx_sender, &config)?;
		let service = Service::new(&context);
		let handle = tokio::spawn(async move { service.run().await });

		let user_transaction = create_signed_transaction(0, &context.config().chain);

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		context
			.mempool_client_sender()
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
