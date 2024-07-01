use crate::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use anyhow::{Context, Result};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_rest_state_root_hash() -> Result<()> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let suzuka_config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;

	let node_url = suzuka_config.execution_config.maptos_config.client.get_rest_url()?;
	let faucet_url = suzuka_config.execution_config.maptos_config.client.get_faucet_url()?;

	let rest_client = Client::new(node_url.clone());
	let faucet_client = FaucetClient::new(faucet_url.clone(), node_url.clone());

	let coin_client = CoinClient::new(&rest_client);
	// Create two accounts locally, Alice and Bob.
	let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng);

	// Print account addresses.
	println!("\n=== Addresses ===");
	println!("Alice: {}", alice.address().to_hex_literal());
	println!("Bob: {}", bob.address().to_hex_literal());

	// Create the accounts on chain, but only fund Alice.
	faucet_client
		.fund(alice.address(), 100_000_000)
		.await
		.context("Failed to fund Alice's account")?;
	faucet_client
		.fund(bob.address(), 100_000_000)
		.await
		.context("Failed to fund Bob's account")?; // <:!:section_3

	// Print initial balances.
	println!("\n=== Initial Balances ===");
	println!(
		"Alice: {:?}",
		coin_client
			.get_account_balance(&alice.address())
			.await
			.context("Failed to get Alice's account balance")?
	);
	println!(
		"Bob: {:?}",
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
	println!("\n=== Intermediate Balances ===");
	// :!:>section_4
	println!(
		"Alice: {:?}",
		coin_client
			.get_account_balance(&alice.address())
			.await
			.context("Failed to get Alice's account balance the second time")?
	);
	println!(
		"Bob: {:?}",
		coin_client
			.get_account_balance(&bob.address())
			.await
			.context("Failed to get Bob's account balance the second time")?
	); // <:!:section_4

	// Have Alice send Bob some more coins.
	let txn_hash = coin_client
		.transfer(&mut alice, bob.address(), 1_000, None)
		.await
		.context("Failed to submit transaction to transfer coins")?; // <:!:section_5
															 // :!:>section_6
	rest_client
		.wait_for_transaction(&txn_hash)
		.await
		.context("Failed when waiting for the transfer transaction")?; // <:!:section_6

	// Print final balances.
	println!("\n=== Final Balances ===");
	println!(
		"Alice: {:?}",
		coin_client
			.get_account_balance(&alice.address())
			.await
			.context("Failed to get Alice's account balance the second time")?
	);
	println!(
		"Bob: {:?}",
		coin_client
			.get_account_balance(&bob.address())
			.await
			.context("Failed to get Bob's account balance the second time")?
	);

	sleep(Duration::from_secs(10)).await;

	let cur_blockheight = rest_client.get_ledger_information().await?.state().block_height;
	let state_root_hash_query = format!("movement/v1/state-root-hash/{}", cur_blockheight);
	let state_root_hash_url = format!("{}{}", node_url, state_root_hash_query);
	println!("State root hash url: {}", state_root_hash_url);

	let client = reqwest::Client::new();
	let health_url = format!("{}movement/v1/health", node_url);
	let response = client.get(&health_url).send().await?;
	println!("response:{response:?}",);
	assert!(response.status().is_success());

	println!("Health check passed");

	let response = client.get(&state_root_hash_url).send().await?;
	println!("response:{response:?}",);
	let state_key = response.text().await?;
	println!("State key: {}", state_key);

	Ok(())
}
