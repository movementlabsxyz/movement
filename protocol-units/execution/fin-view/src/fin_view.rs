use aptos_api::{
	runtime::{get_api_service, get_apis, Apis},
	Context,
};
use aptos_config::config::NodeConfig;
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::{finality_view::FinalityView as AptosFinalityView, DbReader};
use maptos_execution_util::config::Config;

use poem::{http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server};
use tracing::info;

use std::{fmt::format, sync::Arc};

#[derive(Clone)]
/// The API view into the finalized state of the chain.
pub struct FinalityView {
	inner: Arc<AptosFinalityView<Arc<dyn DbReader>>>,
	context: Arc<Context>,
	listen_url: String,
}

impl FinalityView {
	/// Create a new `FinalityView` instance.
	pub fn new(
		inner: Arc<AptosFinalityView<Arc<dyn DbReader>>>,
		context: Arc<Context>,
		listen_url: String,
	) -> Self {
		Self { inner, context, listen_url }
	}

	pub fn try_from_config(
		db_reader: Arc<dyn DbReader>,
		mempool_client_sender: MempoolClientSender,
		config: Config,
	) -> Result<Self, anyhow::Error> {
		let node_config = NodeConfig::default();
		let inner = Arc::new(AptosFinalityView::new(db_reader));
		let context = Arc::new(Context::new(
			config.chain.maptos_chain_id,
			inner.clone(),
			mempool_client_sender,
			node_config,
			None,
		));
		let listen_url =
			format!("{}:{}", config.fin.fin_rest_listen_hostname, config.fin.fin_rest_listen_port,);
		Ok(Self::new(inner, context, listen_url))
	}

	/// Update the finalized view with the latest block height.
	///
	/// The block must be found on the committed chain.
	pub fn set_finalized_block_height(&self, height: u64) -> Result<(), anyhow::Error> {
		self.inner.set_finalized_block_height(height)?;
		Ok(())
	}

	pub fn get_apis(&self) -> Apis {
		get_apis(self.context.clone())
	}

	pub async fn run_service(&self) -> Result<(), anyhow::Error> {
		info!("Starting maptos-fin-view services at: {:?}", self.listen_url);

		let api_service =
			get_api_service(self.context.clone()).server(format!("http://{:?}", self.listen_url));

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
	use aptos_api::accept_type::AcceptType;
	use aptos_sdk::crypto::HashValue;
	use aptos_sdk::transaction_builder::TransactionFactory;
	use aptos_sdk::types::{account_config::aptos_test_root_address, AccountKey, LocalAccount};
	use aptos_types::block_executor::partitioner::{ExecutableBlock, ExecutableTransactions};
	use aptos_types::block_metadata::BlockMetadata;
	use aptos_types::transaction::signature_verified_transaction::SignatureVerifiedTransaction;
	use aptos_types::transaction::Transaction;
	use maptos_opt_executor::Executor;
	use rand::prelude::*;

	#[tokio::test]
	async fn test_set_finalized_block_height_get_api() -> Result<(), anyhow::Error> {
		// Create an Executor and a FinalityView instance from the environment configuration.
		let config = Config::default();
		let executor = Executor::try_from_config(config.clone())?;
		let finality_view = FinalityView::try_from_config(
			executor.db.reader.clone(),
			executor.mempool_client_sender.clone(),
			config,
		)?;

		// Initialize a root account using a predefined keypair and the test root address.
		let root_account = LocalAccount::new(
			aptos_test_root_address(),
			AccountKey::from_private_key(config.chain.maptos_private_key.clone()),
			0,
		);

		// Seed for random number generator, used here to generate predictable results in a test environment.
		let seed = [3u8; 32];
		let mut rng = ::rand::rngs::StdRng::from_seed(seed);

		// Create a transaction factory with the chain ID of the executor.
		let tx_factory = TransactionFactory::new(config.chain.maptos_chain_id.clone());

		let mut account_addrs = Vec::new();

		// Simulate the execution of multiple blocks.
		for _ in 0..3 {
			let (epoch, round) = executor.get_next_epoch_and_round().await?;

			let block_id = HashValue::random(); // Generate a random block ID for each block.

			// Clone the signer from the executor for signing the metadata.
			let signer = executor.signer.clone();
			// Get the current time in microseconds for the block timestamp.
			let current_time_micros = chrono::Utc::now().timestamp_micros() as u64;

			// Create a block metadata transaction.
			let block_metadata = Transaction::BlockMetadata(BlockMetadata::new(
				block_id,
				epoch,
				round,
				signer.author(),
				vec![],
				vec![],
				current_time_micros,
			));

			// Generate new accounts and create transactions for each block.
			let mut transactions = Vec::new();
			transactions.push(block_metadata.clone());

			// Each block will contain a transaction creating an account.
			let new_account = LocalAccount::generate(&mut rng);
			account_addrs.push(new_account.address());

			let user_account_creation_tx = root_account.sign_with_transaction_builder(
				tx_factory.create_user_account(new_account.public_key()),
			);
			transactions.push(Transaction::UserTransaction(user_account_creation_tx));

			// Group all transactions into an unsharded block for execution.
			let executable_transactions = ExecutableTransactions::Unsharded(
				transactions.into_iter().map(SignatureVerifiedTransaction::Valid).collect(),
			);
			let block = ExecutableBlock::new(block_id.clone(), executable_transactions);
			executor.execute_block(block).await?;
		}

		finality_view.set_finalized_block_height(2)?;

		// Retrieve the executor's API interface and fetch the accounts
		let apis = finality_view.get_apis();

		apis.accounts
			.get_account_inner(AcceptType::Bcs, account_addrs[1].into(), None)
			.await
			.expect("account created at block height 2 should be retrieved");
		let res = apis
			.accounts
			.get_account_inner(AcceptType::Bcs, account_addrs[2].into(), None)
			.await;
		assert!(res.is_err(), "account created at block height 3 should not be retrieved");

		Ok(())
	}
}
