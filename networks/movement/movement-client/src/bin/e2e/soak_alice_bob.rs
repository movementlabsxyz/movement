use anyhow::Result;
use aptos_sdk::{
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use movement_client::load_soak_testing::util::add_account_to_accounts_file;
use movement_client::load_soak_testing::util::create_signed_transfer_transaction;
use movement_client::load_soak_testing::util::get_account_from_list;
use movement_client::load_soak_testing::util::FAUCET_URL;
use movement_client::load_soak_testing::util::NODE_URL;
use movement_client::load_soak_testing::TestKind;
use movement_client::load_soak_testing::{execute_test, init_test, ExecutionConfig, Scenario};
use std::path::Path;
use std::sync::Arc;
use tracing::info;

fn main() {
	// Define the Test config. Use the default parameters.
	let mut config = ExecutionConfig::default();

	// Set some params for the load test, drive some requests
	config.kind = TestKind::Soak {
		min_scenarios: 200,
		max_scenarios: 200,
		duration: std::time::Duration::from_secs(12000), // 10 minutes
		number_cycle: 1,
	};

	// 2 Clients and 10 Requests per client
	config.scenarios_per_client = 40;

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
	alice_sequence_number: u64,
}

impl BasicScenario {
	pub fn new(id: usize) -> Self {
		BasicScenario { id, alice: None, bob: None, alice_sequence_number: 0 }
	}
}

#[async_trait::async_trait]
impl Scenario for BasicScenario {
	async fn prepare(&mut self) -> Result<(), anyhow::Error> {
		let rest_client = Client::new(NODE_URL.clone());
		let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
		let faucet_client = faucet_client.with_auth_token("notreal".to_string());

		if std::env::var_os("TEST_ALICE_BOB_ACCOUNT_READ_FROM_FILE").is_some() {
			//bob always 0. Only use to receive transfer.
			let (priv_key, address) = get_account_from_list(0)?;
			let sequence_number = match rest_client.get_account(address).await {
				Ok(response) => response.into_inner().sequence_number,
				Err(_) => {
					tracing::warn!("File private_key:{priv_key} not created, create it.");
					faucet_client.fund(address, 100_000_000_000).await.unwrap();
					0
				}
			};
			self.bob = Some(LocalAccount::new(address, priv_key, sequence_number));
			// Alice account is the scenario id   + 1 (because bob is 0)
			let (priv_key, address) = get_account_from_list(self.id + 1)?;
			let sequence_number = match rest_client.get_account(address).await {
				Ok(response) => response.into_inner().sequence_number,
				Err(_) => {
					tracing::warn!("File private_key:{priv_key} not created, create it.");
					faucet_client.fund(address, 100_000_000_000).await.unwrap();
					0
				}
			};
			info!("Scenario:{} prepare done. account loaded", self.id,);
			self.alice = Some(LocalAccount::new(address, priv_key, sequence_number));
			self.alice_sequence_number = sequence_number;
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

	async fn run(&mut self) -> Result<()> {
		let rest_client = Client::new(NODE_URL.clone());

		let chain_id: u8 = std::env::var_os("TEST_ALICE_BOB_CHAIN_ID")
			.map(|str| str.to_string_lossy().into_owned())
			.map(|val| val.parse().unwrap_or(27))
			.unwrap_or(27);

		let alice = self.alice.as_mut().unwrap();
		let bob = self.bob.as_mut().unwrap();

		for index in 0..50 {
			let pending_tx = create_signed_transfer_transaction(
				chain_id,
				&alice,
				bob.address(),
				100,
				self.alice_sequence_number,
			)
			.await?;
			let tx_hash = rest_client
				.submit(&pending_tx)
				.await
				.expect(&format!(
					"ERROR scenario:{} sequencer number:{}",
					self.id, self.alice_sequence_number
				))
				.into_inner();
			//			tracing::info!("scenario:{} sequencer number:{}", self.id, self.alice_sequence_number);
			self.alice_sequence_number += 1;
			rest_client
				.wait_for_transaction(&tx_hash)
				.await
				.map_err(|err| {
					anyhow::anyhow!("Alice Tx failed:{pending_tx:?} index:{index} because {err}")
				})
				.unwrap();
		}

		// Print final balances.
		// tracing::info!(
		// 	"Scenario:{}\n=== Final Balances ===\n Alice: {:?}\n Bob: {:?}",
		// 	self.id,
		// 	coin_client
		// 		.get_account_balance(&alice.address())
		// 		.await
		// 		.context("Failed to get Alice's account balance the second time")?,
		// 	coin_client
		// 		.get_account_balance(&bob.address())
		// 		.await
		// 		.context("Failed to get Bob's account balance the second time")?
		// );

		Ok(())
	}
}
