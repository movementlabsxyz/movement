use anyhow::{Context, Result};
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use load_soak_testing::execute_test;
use load_soak_testing::init_test;
use load_soak_testing::ExecutionConfig;
use load_soak_testing::Scenario;
use std::str::FromStr;
use url::Url;

fn main() {
	// Define the Test config. Use the default parameters.
	let config = ExecutionConfig::default();

	// Init the Test before execution
	if let Err(err) = init_test(&config) {
		println!("Test init fail ; {err}",);
	}

	// Execute the test.
	let result = execute_test(config, &create_scenario);
	tracing::info!("End Test with result {result:?}",);
}

// Scenario constructor function use by the Test runtime to create new scenarios.
fn create_scenario(id: usize) -> Box<dyn Scenario> {
	Box::new(BasicScenario { id })
}

pub struct BasicScenario {
	id: usize,
}

impl BasicScenario {
	pub fn new(id: usize) -> Self {
		BasicScenario { id }
	}
}

// Scenario trait implementation.
#[async_trait::async_trait]
impl Scenario for BasicScenario {
	async fn run(self: Box<Self>) -> Result<()> {
		let _ =
			tokio::time::sleep(tokio::time::Duration::from_millis(1000 * (self.id as u64))).await;

		let suzuka_config = maptos_execution_util::config::Config::try_from_env()
			.context("Failed to create the suzuka_config")?;
		let node_url = Url::from_str(
			format!("http://{}", suzuka_config.aptos_config.aptos_rest_listen_url.as_str())
				.as_str(),
		)
		.unwrap();

		let faucet_url = Url::from_str(
			format!("http://{}", suzuka_config.aptos_config.aptos_faucet_listen_url.as_str())
				.as_str(),
		)
		.unwrap();

		// :!:>section_1a
		let rest_client = Client::new(node_url.clone());
		let faucet_client = FaucetClient::new(faucet_url.clone(), node_url.clone()); // <:!:section_1a

		// :!:>section_1b
		let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

		// Create two accounts locally, Alice and Bob.
		// :!:>section_2
		let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
		let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

		// Print account addresses.
		tracing::info!(
			"Scenario:{}\n=== Addresses ===\nAlice: {}\nBob: {}",
			self.id,
			alice.address().to_hex_literal(),
			bob.address().to_hex_literal()
		);

		// Create the accounts on chain, but only fund Alice.
		// :!:>section_3
		faucet_client.fund(alice.address(), 100_000_000).await?;
		//			.context("Failed to fund Alice's account")?;
		faucet_client.create_account(bob.address()).await?;
		//			.context("Failed to fund Bob's account")?; // <:!:section_3

		// Print initial balances.
		tracing::info!(
			"Scenario:{}\n=== Initial Balances ===\nAlice: {:?}\nBob: {:?}",
			self.id,
			coin_client
				.get_account_balance(&alice.address())
				.await
				.context("Failed to get Alice's account balance")?,
			coin_client
				.get_account_balance(&bob.address())
				.await
				.context("Failed to get Bob's account balance")?
		);

		// Have Alice send Bob some coins.
		let txn_hash = coin_client
			.transfer(&mut alice, bob.address(), 1_000, None)
			.await
			.context("Failed to submit transaction to transfer coins")?;
		rest_client
			.wait_for_transaction(&txn_hash)
			.await
			.context("Failed when waiting for the transfer transaction")?;

		// Print intermediate balances.
		tracing::info!(
			"Scenario:{}\n=== Intermediate Balances ===\nAlice: {:?}\nBob: {:?}",
			self.id,
			coin_client
				.get_account_balance(&alice.address())
				.await
				.context("Failed to get Alice's account balance the second time")?,
			coin_client
				.get_account_balance(&bob.address())
				.await
				.context("Failed to get Bob's account balance the second time")?
		);

		self.log_exec_info(&format!("Scenario:{} ended", self.id));

		Ok(())
	}
	fn get_id(&self) -> usize {
		self.id
	}
}
