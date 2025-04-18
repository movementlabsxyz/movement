use anyhow::{Context, Result};
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use movement_client::load_soak_testing::{execute_test, init_test, ExecutionConfig, Scenario};
use once_cell::sync::Lazy;
use tracing::info;
use url::Url;

use std::str::FromStr;
use std::sync::Arc;

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
	#[tracing::instrument(skip(self), fields(scenario = self.id))]
	async fn prepare(&mut self) -> Result<(), anyhow::Error> {
		let rest_client = Client::new(NODE_URL.clone());
		let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
		let coin_client = CoinClient::new(&rest_client);

		// Create two accounts locally, Alice and Bob.
		let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
		let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

		// Print account addresses.
		info!(
			address_alice = %alice.address().to_hex_literal(),
			address_bob = %bob.address().to_hex_literal(),
			"accounts created",
		);

		// Create the accounts on chain, but only fund Alice.
		faucet_client.fund(alice.address(), 100_000_000_000).await?;
		faucet_client.create_account(bob.address()).await?;

		// Have Alice send Bob some coins.
		let pending_tx = coin_client
			.transfer(&mut alice, bob.address(), 1_000_000, None)
			.await
			.context("Prepare Failed to submit transaction to transfer coins")?;

		info!(tx_hash = %pending_tx.hash, "waiting for transaction");
		rest_client
			.wait_for_transaction(&pending_tx)
			.await
			.context("Prepare Failed when waiting for the transfer transaction")?;
		info!("Scenario:{} prepare done. account created and founded", self.id,);

		self.alice = Some(alice);
		self.bob = Some(bob);
		Ok(())
	}

	#[tracing::instrument(skip(self), fields(scenario = self.id))]
	async fn run(&mut self) -> Result<()> {
		let rest_client = Client::new(NODE_URL.clone());
		let coin_client = CoinClient::new(&rest_client); // Print initial balances.

		let alice = self.alice.as_mut().unwrap();
		let bob = self.bob.as_mut().unwrap();

		for index in 0..2 {
			// Have Bob send Alice some coins.
			info!("Scenario:{} Before Sent Tx Alice -> Bob index:{index}", self.id);
			let pending_tx = coin_client
				.transfer(bob, alice.address(), 10, None)
				.await
				//				.context("Failed to submit transaction to transfer coins")
				.map_err(|err| anyhow::anyhow!("Alice Tx sublit failed because {err}"))?;
			info!(scenario = %self.id, tx_hash = %pending_tx.hash, index = %index, "waiting for Bob -> Alice transfer to complete");

			rest_client.wait_for_transaction(&pending_tx).await.map_err(|err| {
				anyhow::anyhow!("Alice Tx failed:{pending_tx:?} index:{index} because {err}")
			})?;

			// Have Alice send Bob some more coins.
			info!("Scenario:{} Before Sent Tx Bob -> Alice index:{index}", self.id);
			let pending_tx = coin_client
				.transfer(alice, bob.address(), 10, None)
				.await
				//				.context("Failed to submit transaction to transfer coins")
				.map_err(|err| {
					anyhow::anyhow!("Bob Tx submit index:{index} failed because {err}")
				})?;
			info!(scenario = %self.id, tx_hash = %pending_tx.hash, index = %index, "waiting for Alice -> Bob transfer to complete");

			rest_client.wait_for_transaction(&pending_tx).await.map_err(|err| {
				anyhow::anyhow!("Bob Tx failed:{pending_tx:?} index:{index} because {err}")
			})?;
		}

		// Print final balances.
		info!(
			"final balances, Alice: {:?}, Bob: {:?}",
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
