use anyhow::{Context, Result};
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use movement_client::load_soak_testing::util::add_account_to_accounts_file;
use movement_client::load_soak_testing::util::create_signed_transfer_transaction;
use movement_client::load_soak_testing::util::get_account_from_list;
use movement_client::load_soak_testing::util::FAUCET_URL;
use movement_client::load_soak_testing::util::NODE_URL;
use movement_client::load_soak_testing::{execute_test, init_test, ExecutionConfig, Scenario};
use std::path::Path;
use std::sync::Arc;
use tracing::info;

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

#[async_trait::async_trait]
impl Scenario for BasicScenario {
	#[tracing::instrument(skip(self), fields(scenario = self.id))]
	async fn prepare(&mut self) -> Result<(), anyhow::Error> {
		info!(
			"Scenario:{} prepare start. NODE_URL:{} FAUCET_URL:{}",
			self.id,
			&NODE_URL.to_string(),
			&FAUCET_URL.to_string()
		);

		let rest_client = Client::new(NODE_URL.clone());
		let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
		//let faucet_client = faucet_client.with_auth_token("notreal".to_string());

		if std::env::var_os("TEST_ALICE_BOB_ACCOUNT_READ_FROM_FILE").is_some() {
			//bob always 0. Only use to receive transfer.
			let (priv_key, address) = get_account_from_list(0)?;
			let sequence_number = match rest_client.get_account(address).await {
				Ok(response) => response.into_inner().sequence_number,
				Err(_) => {
					tracing::warn!("Bod account:{address} not created, create it.");
					faucet_client.fund(address, 100_000_000_000).await.unwrap();
					tracing::warn!("Bod account:{address} funded.");
					0
				}
			};
			info!(
				"Scenario:{} prepare. Bob Account:{address} sequence_number:{sequence_number}",
				self.id,
			);
			self.bob = Some(LocalAccount::new(address, priv_key, sequence_number));
			// Alice account is the scenario id   + 1 (because bob is 0)
			let (priv_key, address) = get_account_from_list(self.id + 1)?;
			let sequence_number = match rest_client.get_account(address).await {
				Ok(response) => response.into_inner().sequence_number,
				Err(_) => {
					tracing::warn!("Alice account:{address} not created, create it.");
					faucet_client.fund(address, 100_000_000_000).await.unwrap();
					0
				}
			};
			info!("Scenario:{} prepare done. account loaded", self.id,);
			self.alice = Some(LocalAccount::new(address, priv_key, sequence_number));
		} else {
			//create and fund a new account.

			// Create two accounts locally, Alice and Bob.
			let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
			let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

			// Print account addresses.
			info!(
				address_alice = %alice.address().to_hex_literal(),
				address_bob = %bob.address().to_hex_literal(),
				"accounts created",
			);

			// Create the accounts on chain, but only fund Alice.
			faucet_client.fund(alice.address(), 100_000_000_000).await.unwrap();
			faucet_client.create_account(bob.address()).await.unwrap();
			info!("Scenario:{} prepare done. account created and founded", self.id,);

			if let Some(account_file) = std::env::var_os("TEST_ALICE_BOB_ACCOUNT_FILE_PATH") {
				let account_file = Path::new(&account_file);
				add_account_to_accounts_file(
					account_file,
					alice.public_key(),
					alice.private_key(),
				)?;
			}

			self.alice = Some(alice);
			self.bob = Some(bob);
		}

		Ok(())
	}

	#[tracing::instrument(skip(self), fields(scenario = self.id))]
	async fn run(&mut self) -> Result<()> {
		let rest_client = Client::new(NODE_URL.clone());
		let coin_client = CoinClient::new(&rest_client); // Print initial balances.

		let alice = self.alice.as_mut().unwrap();
		let bob = self.bob.as_mut().unwrap();

		let chain_id: u8 = std::env::var_os("TEST_ALICE_BOB_CHAIN_ID")
			.map(|str| str.to_string_lossy().into_owned())
			.map(|val| val.parse().unwrap_or(27))
			.unwrap_or(27);

		let mut sequence_number = alice.sequence_number();

		info!("Scenario:{} Start sending transactions.", self.id,);

		for index in 0..2 {
			//			let _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
			// Have Alice send Bob some more coins.
			// let pending_tx = coin_client
			// 	.transfer(alice, bob.address(), 10, None)
			// 	.await
			// 	.map_err(|err| anyhow::anyhow!("Alice Tx submit failed because {err}"))?;

			let pending_tx = create_signed_transfer_transaction(
				chain_id,
				&alice,
				bob.address(),
				100,
				sequence_number,
			)
			.await?;
			let tx_hash = rest_client.submit(&pending_tx).await.unwrap().into_inner();

			sequence_number += 1;

			info!(scenario = %self.id, tx_hash = ?tx_hash, index = %index, "waiting for Alice -> Bod transfer to complete");

			rest_client
				.wait_for_transaction(&tx_hash)
				.await
				.map_err(|err| {
					anyhow::anyhow!("Alice Tx failed:{pending_tx:?} index:{index} because {err}")
				})
				.unwrap();
			info!("Scenario:{} Transaction executed.", self.id,);
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
