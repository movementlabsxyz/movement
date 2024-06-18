use crate::{
	coin_client::CoinClient,
	rest_client::{
		// aptos_api_types::TransactionOnChainData,
		Client,
		FaucetClient,
	},
	types::{chain_id::ChainId, LocalAccount},
};
use anyhow::Context;
use aptos_sdk::crypto::ed25519::Ed25519PublicKey;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::{
	crypto::ed25519::Ed25519PrivateKey,
	// 	rest_client::Account
};
use buildtime_helpers::cargo::cargo_workspace;
use commander::run_command;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::time::{sleep, Duration};
use std::{thread, time};
use url::Url;
// use aptos_sdk::move_types::ident_str;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
// use aptos_sdk::move_types::language_storage::StructTag;
// use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_sdk::transaction_builder::TransactionBuilder;
use aptos_sdk::types::account_address::AccountAddress;
// use aptos_sdk::types::move_utils::MemberId;
use aptos_sdk::types::transaction::authenticator::AuthenticationKey;
use aptos_sdk::types::transaction::EntryFunction;
use aptos_sdk::types::transaction::TransactionPayload;
// use aptos_sdk::transaction_builder::TransactionFactory;
use std::time::{SystemTime, UNIX_EPOCH};

static SUZUKA_CONFIG: Lazy<suzuka_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>().unwrap();
	config
});

// :!:>section_1c
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
// <:!:section_1c

#[tokio::test]
async fn test_example_interaction() -> Result<(), anyhow::Error> {
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
	println!("State root hash url: {}", state_root_hash_url);

	let client = reqwest::Client::new();

	let health_url = format!("{}/movement/v1/health", base_url);
	let response = client.get(&health_url).send().await?;
	assert!(response.status().is_success());

	println!("Health check passed");

	let response = client.get(&state_root_hash_url).send().await?;
	let state_key = response.text().await?;
	println!("State key: {}", state_key);

	Ok(())
}

#[derive(Debug, Deserialize)]
struct Config {
	profiles: Profiles,
}

#[derive(Debug, Deserialize)]
struct Profiles {
	default: DefaultProfile,
}

#[derive(Debug, Deserialize)]
struct DefaultProfile {
	account: String,
	private_key: String,
}

/*async fn send_tx(
	client: Client,
	chain_id: u8,
	account: LocalAccount,
	module: ModuleId,
	function_name: &str,
	type_args: Vec<TypeTag>,
	args: Vec<Vec<u8>>,
) -> Result<TransactionOnChainData, anyhow::Error> {
	// print the module id
	println!("module: {:?}", module);
	// print the function name
	println!("function_name: {:?}", function_name);

	//get account sequence number
	let account_address = account.address();
	let sequence_number = account.sequence_number();
	let identifier = Identifier::new(function_name)?;
	let payload =
		TransactionPayload::EntryFunction(EntryFunction::new(module, identifier, type_args, args));

	let timeout = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs() + 30;
	let txn_builder = TransactionBuilder::new(payload, timeout, ChainId::new(chain_id))
		.sender(account_address)
		.sequence_number(sequence_number + 1)
		.max_gas_amount(5000)
		.gas_unit_price(100);
	// let raw_tx = txn_builder.build();

	let signed_transaction = account.sign_with_transaction_builder(txn_builder);

	let tx_receipt_data = client.submit_and_wait_bcs(&signed_transaction).await?.inner().clone();
	println!("tx_receipt_data: {tx_receipt_data:?}",);

	Ok::<TransactionOnChainData, anyhow::Error>(tx_receipt_data)
}*/

#[tokio::test]
pub async fn test_complex_alice() -> Result<(), anyhow::Error> {
	println!("Running test_complex_alice");
	std::env::set_var("NODE_URL", NODE_URL.clone().as_str());
	std::env::set_var("FAUCET_URL", FAUCET_URL.clone().as_str());

	// Get the root path of the cargo workspace
	let root: PathBuf = cargo_workspace()?;
	let additional_path = "networks/suzuka/suzuka-client/src/tests/complex-alice/";
	let combined_path = root.join(additional_path);

	// Convert the combined path to a string
	let test = combined_path.to_string_lossy();
	println!("{}", test);

	// let args = format!("echo -ne '\\n' | aptos init --network custom --rest-url {} --faucet-url {} --assume-yes", node_url, faucet_url);
	let init_output =
		run_command("/bin/bash", &[format!("{}{}", test, "deploy.sh").as_str()]).await?;
	println!("{}", init_output);

	let one_sec = time::Duration::from_millis(5000);

	thread::sleep(one_sec);

	let yaml_content = fs::read_to_string(".aptos/config.yaml")?;

	let config: Config = serde_yaml::from_str(&yaml_content)?;

	// Access the `account` field
	let module_address = AccountAddress::from_hex_literal(
		format!("0x{}", config.profiles.default.account).as_str(),
	)?;
	let private_key_import = &config.profiles.default.private_key;
	let private_key = Ed25519PrivateKey::from_encoded_string(private_key_import)?;

	let public_key = Ed25519PublicKey::from(&private_key);
	let account_address = AuthenticationKey::ed25519(&public_key).account_address();
	println!("{}", account_address);
	println!("{}", module_address);

	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a
	let chain_id = rest_client.get_index().await?.inner().chain_id;

	// Create two accounts locally, Alice and Bob.
	// :!:>section_2
	let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
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

	//
	println!("Calling with Alice to {:#?}", module_address);
	let bid: u8 = 10;
	let transaction_builder = TransactionBuilder::new(
		TransactionPayload::EntryFunction(EntryFunction::new(
			ModuleId::new(module_address, Identifier::new("resource_roulette").unwrap()),
			Identifier::new("bid").unwrap(),
			vec![],
			vec![bcs::to_bytes(&bid).unwrap()],
		)),
		SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 30,
		ChainId::new(chain_id),
	)
	.sender(alice.address())
	.sequence_number(alice.sequence_number())
	.max_gas_amount(5_000)
	.gas_unit_price(100);

	let signed_transaction = alice.sign_with_transaction_builder(transaction_builder);
	let tx_receipt_data =
		rest_client.submit_and_wait_bcs(&signed_transaction).await?.inner().clone();
	println!("tx_receipt_data: {:?}", tx_receipt_data);

	Ok(())
}
