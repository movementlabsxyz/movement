use super::Executor;
use aptos_api::{
	get_api_service,
	runtime::{get_apis, Apis},
};
use poem::{http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server};
use tracing::info;

impl Executor {
	pub fn get_apis(&self) -> Apis {
		get_apis(self.context())
	}

	pub async fn run_service(&self) -> Result<(), anyhow::Error> {
		info!("Starting maptos-opt-executor services at: {:?}", self.listen_url);

		let api_service =
			get_api_service(self.context()).server(format!("http://{:?}", self.listen_url));

		let ui = api_service.swagger_ui();

		let cors = Cors::new()
			.allow_methods(vec![Method::GET, Method::POST])
			.allow_credentials(true);
		let app = Route::new().nest("/v1", api_service).nest("/spec", ui).with(cors);

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
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519Signature},
		PrivateKey, Uniform,
	};
	use aptos_mempool::MempoolClientRequest;
	use aptos_types::{
		account_address::AccountAddress,
		chain_id::ChainId,
		transaction::{RawTransaction, Script, SignedTransaction, TransactionPayload},
	};
	use futures::channel::oneshot;
	use futures::SinkExt;
	use maptos_execution_util::config::Config;

	fn create_signed_transaction(gas_unit_price: u64, chain_id: ChainId) -> SignedTransaction {
		let private_key = Ed25519PrivateKey::generate_for_testing();
		let public_key = private_key.public_key();
		let transaction_payload = TransactionPayload::Script(Script::new(vec![0], vec![], vec![]));
		let raw_transaction = RawTransaction::new(
			AccountAddress::random(),
			0,
			transaction_payload,
			0,
			gas_unit_price,
			0,
			chain_id, // This is the value used in aptos testing code.
		);
		SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
	}

	#[tokio::test]
	async fn test_pipe_mempool_while_server_running() -> Result<(), anyhow::Error> {
		let mut executor = Executor::try_test_default()?;
		let server_executor = executor.clone();

		let handle = tokio::spawn(async move {
			server_executor.run_service().await?;
			Ok(()) as Result<(), anyhow::Error>
		});

		let user_transaction = create_signed_transaction(0, executor.maptos_config.chain.maptos_chain_id.clone());

		// send transaction to mempool
		let (req_sender, callback) = oneshot::channel();
		executor
			.mempool_client_sender
			.send(MempoolClientRequest::SubmitTransaction(user_transaction.clone(), req_sender))
			.await?;

		// tick the transaction pipe
		let (tx, rx) = async_channel::unbounded();
		executor.tick_transaction_pipe(tx).await?;

		// receive the callback
		callback.await??;

		// receive the transaction
		let received_transaction = rx.recv().await?;
		assert_eq!(received_transaction, user_transaction);

		handle.abort();

		Ok(())
	}
}
