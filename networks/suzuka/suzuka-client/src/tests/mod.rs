use crate::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use alloy_network::EthereumSigner;
use alloy_primitives::Address;
use alloy_provider::ProviderBuilder;
use alloy_signer_wallet::LocalWallet;
use anyhow::anyhow;
use anyhow::{Context, Result};
use mcr_settlement_client::{
	eth_client::{McrEthSettlementClient, McrEthSettlementConfig},
	McrSettlementClientOperations,
};
use once_cell::sync::Lazy;
use std::str::FromStr;
use tokio::time::{sleep, Duration};
use url::Url;

mod settlement_state;

static SUZUKA_CONFIG: Lazy<maptos_execution_util::config::Config> = Lazy::new(|| {
	maptos_execution_util::config::Config::try_from_env()
		.context("Failed to create the config")
		.unwrap()
});

// :!:>section_1c
static NODE_URL: Lazy<Url> = Lazy::new(|| {
	Url::from_str(
		format!("http://{}", SUZUKA_CONFIG.aptos_config.aptos_rest_listen_url.as_str()).as_str(),
	)
	.unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
	Url::from_str(
		format!("http://{}", SUZUKA_CONFIG.aptos_config.aptos_faucet_listen_url.as_str()).as_str(),
	)
	.unwrap()
});
// <:!:section_1c

#[tokio::test]
async fn test_example_interaction() -> Result<()> {
	const MAX_TX_SEND_RETRY: usize = 10;
	const DEFAULT_TX_GAS_LIMIT: u128 = 10_000_000_000_000_000;

	// :!:>section_1a
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

	// :!:>section_1b
	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

	// Create two accounts locally, Alice and Bob.
	// :!:>section_2
	let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

	// Print account addresses.
	println!("\n=== Addresses ===");
	println!("Alice: {}", alice.address().to_hex_literal());
	println!("Bob: {}", bob.address().to_hex_literal());

	// Create the accounts on chain, but only fund Alice.
	// :!:>section_3
	faucet_client
		.fund(alice.address(), 100_000_000)
		.await
		.context("Failed to fund Alice's account")?;
	faucet_client
		.create_account(bob.address())
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
	// :!:>section_5
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

	let anvil_rpc_port = "8545";
	let anvil_rpc_url = format!("http://localhost:{anvil_rpc_port}");
	let anvil_ws_url = format!("ws://localhost:{anvil_rpc_port}");

	let cur_blockheight = rest_client.get_ledger_information().await?.state().block_height;
	let base_url = "http://localhost:30731";
	let state_root_hash_query = format!("/movement/v1/state-root-hash/{}", cur_blockheight);
	let state_root_hash_url = format!("{}{}", base_url, state_root_hash_query);

	let client = reqwest::Client::new();

	let health_url = format!("{}/movement/v1/health", base_url);
	let response = client.get(&health_url).send().await?;
	assert!(response.status().is_success());

	println!("Health check passed");

	let response = client.get(&state_root_hash_url).send().await?;
	let state_key = response.text().await?;
	println!("State key: {}", state_key);

	let mcr_address = read_mcr_sc_adress()?;
	let anvil_address = read_anvil_json_file_address()?;
	let signer: LocalWallet = anvil_address[1].1.parse()?;

	println!("MCR address: {}", mcr_address);

	//Build client 1 and send first commitment.
	let provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.signer(EthereumSigner::from(signer.clone()))
		.on_http(anvil_rpc_url.parse().unwrap());

	let config = McrEthSettlementConfig {
		mrc_contract_address: mcr_address.to_string(),
		gas_limit: DEFAULT_TX_GAS_LIMIT,
		tx_send_nb_retry: MAX_TX_SEND_RETRY,
	};

	let eth_client = McrEthSettlementClient::build_with_provider(
		provider,
		signer.address(),
		anvil_ws_url,
		config.clone(),
	)
	.await?;

	if let Some(commitment) = eth_client.get_commitment_at_height(cur_blockheight).await? {
		assert_eq!(commitment.commitment.to_string(), state_key);
	} else {
		return Err(anyhow!("No commitment found at block height {}", cur_blockheight));
	}

	Ok(())
}

fn read_mcr_sc_adress() -> Result<Address, anyhow::Error> {
	let file_path = std::env::var("MCR_SC_ADDRESS_FILE")?;
	let addr_str = std::fs::read_to_string(file_path)?;
	let addr: Address = addr_str.trim().parse()?;
	Ok(addr)
}

fn read_anvil_json_file_address() -> Result<Vec<(String, String)>, anyhow::Error> {
	use serde_json::{from_str, Value};

	let anvil_conf_file = std::env::var("ANVIL_JSON_PATH")?;
	let file_content = std::fs::read_to_string(anvil_conf_file)?;

	let json_value: Value = from_str(&file_content)?;

	// Extract the available_accounts and private_keys fields
	let available_accounts_iter = json_value["available_accounts"]
		.as_array()
		.expect("available_accounts should be an array")
		.iter()
		.map(|v| v.as_str().map(|s| s.to_string()))
		.flatten();

	let private_keys_iter = json_value["private_keys"]
		.as_array()
		.expect("private_keys should be an array")
		.iter()
		.map(|v| v.as_str().map(|s| s.to_string()))
		.flatten();

	let res = available_accounts_iter
		.zip(private_keys_iter)
		.collect::<Vec<(String, String)>>();
	Ok(res)
}
