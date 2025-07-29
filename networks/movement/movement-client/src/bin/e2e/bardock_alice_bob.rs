use anyhow::{Context, Result};
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use movement_client::load_soak_testing::{
	execute_test, init_test, ExecutionConfig, Scenario, TestKind,
};
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::{str::FromStr, time::Duration};
use tokio::time::sleep;
use url::Url;

fn main() {
	let mut config = ExecutionConfig::default();

	// Set some params for the load test, drive some requests
	config.kind = TestKind::Soak {
		min_scenarios: 20,
		max_scenarios: 20,
		duration: std::time::Duration::from_secs(600), // 10 minutes
		number_cycle: 1,
	};

	config.scenarios_per_client = 20; // 20Client Requests

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
		format!("https://{}:{}", node_connection_address, node_connection_port);

	Url::from_str(node_connection_url.as_str()).unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
	//The Bardock Faucet Listen URL
	Url::from_str("https://faucet.testnet.bardock.movementnetwork.xyz").unwrap()
});

#[async_trait::async_trait]
impl Scenario for BasicScenario {
	async fn run(&mut self) -> Result<()> {
		// Sleep for 7 seconds before each Scenario run to not get Rate Limited
		sleep(Duration::from_secs(5)).await;

		let rest_client = Client::new(NODE_URL.clone());
		let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
		let coin_client = CoinClient::new(&rest_client);

		// Create two accounts locally, Alice and Bob.
		let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
		let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

		// Print account addresses.
		tracing::info!(
			"Scenario:{}\n=== Addresses ===\nAlice: {}\nBob: {}",
			self.id,
			alice.address().to_hex_literal(),
			bob.address().to_hex_literal()
		);

		tracing::info!("{} Before alice fund", self.id);
		self.log_exec_info(&format!("{} Before alice fund", self.id));
		// Create the accounts on chain, but only fund Alice.
		faucet_client.fund(alice.address(), 100_000_000).await?;
		tracing::info!("{} Before Bod create_account", self.id);
		self.log_exec_info(&format!("{} Before Bod create_account", self.id));
		faucet_client.create_account(bob.address()).await?;
		tracing::info!("{} After Bod create_account", self.id);
		self.log_exec_info(&format!("{} After Bod create_account", self.id));

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

		// Have Alice send Bob some coins.
		let txn_hash = coin_client
			.transfer(&mut alice, bob.address(), 1_000, None)
			.await
			.context("Failed to submit transaction to transfer coins")?;
		tracing::info!("Alice Transfer To Bob tx_hash: {:?}", txn_hash);
		rest_client
			.wait_for_transaction(&txn_hash)
			.await
			.context("Failed when waiting for the transfer transaction")?;

		// Print intermediate balances.
		tracing::info!(
			"Scenario:{}\n=== Intermediate Balances ===\n Alice: {:?}\n Bob: {:?}",
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

		// Have Alice send Bob some more coins.
		let txn_hash = coin_client
			.transfer(&mut alice, bob.address(), 1_000, None)
			.await
			.context("Failed to submit transaction to transfer coins")?; // <:!:section_5
		tracing::info!("Alice Transfer To Bob tx_hash: {:?}", txn_hash);
		// :!:>section_6
		rest_client
			.wait_for_transaction(&txn_hash)
			.await
			.context("Failed when waiting for the transfer transaction")?; // <:!:section_6

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
