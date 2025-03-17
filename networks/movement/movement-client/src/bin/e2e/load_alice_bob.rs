use anyhow::{Context, Result};
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use movement_client::load_soak_testing::{execute_test, init_test, ExecutionConfig, Scenario};
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::sync::Arc;
use url::Url;

fn main() {
	// Define the Test config. Use the default parameters.
	let config = ExecutionConfig::default();

	// Init the Test before execution
	if let Err(err) = init_test(&config) {
		println!("Test init fail ; {err}",);
	}

	// Execute the test.
	let result = execute_test(config, Arc::new(create_scenario));
	tracing::info!("End Test with result {result:?}",);
}

// Scenario constructor function use by the Test runtime to create new scenarios.
fn create_scenario(id: usize) -> Box<dyn Scenario> {
	Box::new(BasicScenario::new(id))
}

pub struct BasicScenario {
	id: usize,
	alice: Option<LocalAccount>,
	bob: Option<LocalAccount>,
}

impl BasicScenario {
	pub fn new(id: usize) -> Self {
		BasicScenario { id, alice: None, bob: None }
	}
}

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let node_connection_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_port
		.clone();

	let node_connection_url =
		format!("http://{}:{}", node_connection_address, node_connection_port);

	Url::from_str(node_connection_url.as_str()).unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
	let faucet_listen_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_hostname
		.clone();
	let faucet_listen_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_port
		.clone();

	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);

	Url::from_str(faucet_listen_url.as_str()).unwrap()
});

#[async_trait::async_trait]
impl Scenario for BasicScenario {
	async fn prepare(&mut self) -> Result<(), anyhow::Error> {
		let rest_client = Client::new(NODE_URL.clone());
		let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
		let coin_client = CoinClient::new(&rest_client);

		// Create two accounts locally, Alice and Bob.
		let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
		let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

		// Print account addresses.
		tracing::info!(
			"Scenario:{} prepare \n=== Addresses ===\nAlice: {}\nBob: {}",
			self.id,
			alice.address().to_hex_literal(),
			bob.address().to_hex_literal()
		);

		// Create the accounts on chain, but only fund Alice.
		faucet_client.fund(alice.address(), 100_000_000_000).await?;
		faucet_client.create_account(bob.address()).await?;

		// Have Alice send Bob some coins.
		let txn_hash = coin_client
			.transfer(&mut alice, bob.address(), 1_000_000, None)
			.await
			.context("Failed to submit transaction to transfer coins")?;
		rest_client
			.wait_for_transaction(&txn_hash)
			.await
			.context("Failed when waiting for the transfer transaction")?;
		tracing::info!("Scenario:{} prepare done. account created and founded", self.id,);

		self.alice = Some(alice);
		self.bob = Some(bob);
		Ok(())
	}

	async fn run(&mut self) -> Result<()> {
		let rest_client = Client::new(NODE_URL.clone());
		let coin_client = CoinClient::new(&rest_client); // Print initial balances.

		let alice = self.alice.as_mut().unwrap();
		let bob = self.bob.as_mut().unwrap();

		for _ in 0..2 {
			// Have Bob send Alice some coins.
			let txn_hash = coin_client
				.transfer(bob, alice.address(), 10, None)
				.await
				.context("Failed to submit transaction to transfer coins")?;
			rest_client
				.wait_for_transaction(&txn_hash)
				.await
				.context("Failed when waiting for the transfer transaction")?;

			// Have Alice send Bob some more coins.
			let txn_hash = coin_client
				.transfer(alice, bob.address(), 10, None)
				.await
				.context("Failed to submit transaction to transfer coins")?;
			rest_client
				.wait_for_transaction(&txn_hash)
				.await
				.context("Failed when waiting for the transfer transaction")?;
		}

		// Print final balances.
		tracing::info!(
			"Scenario:{}\n=== Final Balances ===\n Alice: {:?}\n Bob: {:?}",
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

		Ok(())
	}
}
