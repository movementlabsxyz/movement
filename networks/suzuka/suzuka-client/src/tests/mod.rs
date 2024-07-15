use crate::load_soak_testing::{execute_test, init_test, ExecutionConfig, Scenario, TestKind};
use crate::{
	coin_client::{CoinClient, TransferOptions},
	rest_client::{
		aptos_api_types::{TransactionOnChainData, ViewFunction},
		Client, FaucetClient,
	},
	types::{chain_id::ChainId, LocalAccount},
	transaction_builder::TransactionBuilder,
};
use anyhow::Context;
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::transaction::authenticator::AuthenticationKey;
use aptos_sdk::types::transaction::EntryFunction;
use aptos_sdk::types::transaction::TransactionPayload;
use aptos_sdk::{crypto::ed25519::Ed25519PublicKey, move_types::language_storage::TypeTag};
use buildtime_helpers::cargo::cargo_workspace;
use commander::run_command;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, sync::Arc};
use std::{thread, time};
use url::Url;

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

	// malformed sequence number
	let options = TransferOptions::default();
	let chain_id = rest_client
            .get_index()
            .await
            .context("Failed to get chain ID")?
            .inner()
            .chain_id;
	let transaction_builder = TransactionBuilder::new(
		TransactionPayload::EntryFunction(EntryFunction::new(
			ModuleId::new(AccountAddress::ONE, Identifier::new("coin").unwrap()),
			Identifier::new("transfer").unwrap(),
			vec![TypeTag::from_str(options.coin_type).unwrap()],
			vec![
				bcs::to_bytes(&bob.address()).unwrap(),
				bcs::to_bytes(&(1_000 as u64)).unwrap(),
			],
		)),
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_secs()
			+ options.timeout_secs,
		ChainId::new(chain_id),
	)
	.sender(alice.address())
	.sequence_number(alice.sequence_number())
	.max_gas_amount(options.max_gas_amount)
	.gas_unit_price(options.gas_unit_price);
	let signed_txn = alice.sign_with_transaction_builder(transaction_builder);

	// first send should work
	let txn_hash = rest_client
		.submit(&signed_txn)
		.await
		.context("Failed to submit transfer transaction")?
		.into_inner();
	rest_client.wait_for_transaction(&txn_hash).await.context(
		"Failed when waiting for the transfer transaction with a malformed sequence number",
	)?;

	// second send should fail...
	let txn_hash = rest_client
		.submit(&signed_txn)
		.await
		.context("Failed to submit transfer transaction")?
		.into_inner();
	match rest_client.wait_for_transaction(&txn_hash).await {
		Ok(_) => panic!("Expected transaction to fail"),
		Err(e) => {
			println!("Expected transaction failed: {:?}", e);
		}
	}

	// ...but not crash the node.
	// So, this should work.
	let txn_hash = coin_client
		.transfer(&mut alice, bob.address(), 1_000, None)
		.await
		.context("Failed to submit transaction to transfer coins")?; // <:!:section_5
															 // :!:>section_6
	rest_client
		.wait_for_transaction(&txn_hash)
		.await
		.context("Failed when waiting for the transfer transaction")?;


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

async fn send_tx(
	client: &Client,
	chain_id: u8,
	account: &LocalAccount,
	module_address: AccountAddress,
	module_name: &str,
	function_name: &str,
	type_args: Vec<TypeTag>,
	args: Vec<Vec<u8>>,
) -> Result<TransactionOnChainData, anyhow::Error> {
	let five_sec = time::Duration::from_millis(5000);
	thread::sleep(five_sec);
	let transaction_builder = TransactionBuilder::new(
		TransactionPayload::EntryFunction(EntryFunction::new(
			ModuleId::new(module_address, Identifier::new(module_name).unwrap()),
			Identifier::new(function_name).unwrap(),
			type_args,
			args,
		)),
		SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 30,
		ChainId::new(chain_id),
	)
	.sender(account.address())
	.sequence_number(account.sequence_number())
	.max_gas_amount(5_000)
	.gas_unit_price(100);

	let signed_transaction = account.sign_with_transaction_builder(transaction_builder);
	let tx_receipt_data = client.submit_and_wait_bcs(&signed_transaction).await?.inner().clone();
	Ok(tx_receipt_data)
}

async fn view<T: DeserializeOwned>(
	client: &Client,
	module_address: AccountAddress,
	module_name: &str,
	function_name: &str,
	type_args: Vec<TypeTag>,
	args: Vec<Vec<u8>>,
) -> Vec<T> {
	println!("view call to {}::{}::{}", module_address, module_name, function_name);

	// BCS
	let bcs_view_request = ViewFunction {
		module: ModuleId::new(module_address, Identifier::new(module_name).unwrap()),
		function: Identifier::new(function_name).unwrap(),
		ty_args: type_args,
		args: args,
	};

	// Balance should be 0 and there should only be one return value
	let bcs_ret_values: Vec<T> =
		client.view_bcs(&bcs_view_request, None).await.unwrap().into_inner();
	return bcs_ret_values;
}

#[test]
fn complex_alice_load() {
	let config = ExecutionConfig::default();
	if let Err(err) = init_test(&config) {
		println!("Complex Alice Test init fail {err}",);
	}

	let result = execute_test(config, Arc::new(create_complex_alice_scenario));
	tracing::info!("Complex Alice Test result: {:?}", result);
}

#[test]
fn complex_alice_soak() {
	let mut config = ExecutionConfig::default();
	config.kind = TestKind::Soak {
		min_scenarios: 1,
		max_scenarios: 1,
		duration: std::time::Duration::from_secs(60),
		number_cycle: 1,
	};
	if let Err(err) = init_test(&config) {
		println!("Complex Alice Test init fail {err}",);
	}

	let result = execute_test(config, Arc::new(create_complex_alice_scenario));
	tracing::info!("Complex Alice Test result: {:?}", result);
}

fn create_complex_alice_scenario(_id: usize) -> Box<dyn Scenario> {
	Box::new(ComplexAliceScenario)
}
struct ComplexAliceScenario;

#[async_trait::async_trait]
impl Scenario for ComplexAliceScenario {
	async fn run(self: Box<Self>) -> Result<(), anyhow::Error> {
		test_complex_alice_internal().await
	}
}

#[tokio::test]
pub async fn test_complex_alice() -> Result<(), anyhow::Error> {
	test_complex_alice_internal().await
}

async fn test_complex_alice_internal() -> Result<(), anyhow::Error> {
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

	let five_sec = time::Duration::from_millis(5000);
	thread::sleep(five_sec);

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

	let rest_client = Client::new(NODE_URL.clone());

	let account_client = rest_client.get_account(account_address).await?;
	let sequence_number = account_client.inner().sequence_number;
	println!("{}", account_address);
	println!("{}", module_address);

	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a
	let chain_id = rest_client.get_index().await?.inner().chain_id;

	// Create two accounts locally, Alice and Bob.
	// :!:>section_2
	let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng);
	let deployer = LocalAccount::new(account_address, private_key, sequence_number);

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
		.context("Failed to fund Bob's account")?;

	let empty_type_tag: Vec<TypeTag> = Vec::new();
	match send_tx(
		&rest_client,
		chain_id,
		&alice,
		module_address,
		"resource_roulette",
		"bid",
		empty_type_tag.clone(),
		vec![bcs::to_bytes(&(10 as u8)).unwrap()],
	)
	.await
	{
		Ok(tx1) => println!("Bid with Alice: {:?}", tx1),
		Err(e) => {
			println!("Transaction failed: {:?}", e);
		}
	}

	println!("expected to error");
	match send_tx(
		&rest_client,
		chain_id,
		&bob,
		module_address,
		"resource_roulette",
		"bid",
		empty_type_tag.clone(),
		vec![bcs::to_bytes(&(5 as u8)).unwrap()],
	)
	.await
	{
		Ok(tx2) => println!("Bid with Poor Bob: {:?}", tx2),
		Err(e) => {
			println!("Expected transaction failed: {:?}", e);
		}
	}

	faucet_client
		.fund(bob.address(), 100_000_000)
		.await
		.context("Failed to fund Bob's account")?;

	match send_tx(
		&rest_client,
		chain_id,
		&bob,
		module_address,
		"resource_roulette",
		"bid",
		empty_type_tag.clone(),
		vec![bcs::to_bytes(&(5 as u8)).unwrap()],
	)
	.await
	{
		Ok(tx3) => println!("Bid with Bob: {:?}", tx3),
		Err(e) => {
			println!("Transaction failed: {:?}", e);
		}
	}

	let get_noise = view::<u64>(
		&rest_client,
		module_address,
		"resource_roulette",
		"get_noise",
		empty_type_tag.clone(),
		vec![],
	)
	.await;
	println!("Noise: {:?}", get_noise);
	let total_bids = view::<u64>(
		&rest_client,
		module_address,
		"resource_roulette",
		"total_bids",
		empty_type_tag.clone(),
		vec![],
	)
	.await;
	println!("Total Bids: {:?}", total_bids);

	match send_tx(
		&rest_client,
		chain_id,
		&deployer,
		module_address,
		"resource_roulette",
		"bid",
		empty_type_tag.clone(),
		vec![bcs::to_bytes(&(10 as u8)).unwrap()],
	)
	.await
	{
		Ok(tx4) => println!("Bid with Deployer: {:?}", tx4),
		Err(e) => {
			println!("Transaction failed: {:?}", e);
		}
	}

	println!("expected to error");
	match send_tx(
		&rest_client,
		chain_id,
		&bob,
		module_address,
		"resource_roulette",
		"spin",
		empty_type_tag.clone(),
		vec![],
	)
	.await
	{
		Ok(tx5) => println!("Spin with Bon: {:?}", tx5),
		Err(e) => {
			println!("Expected transaction failed: {:?}", e);
		}
	}

	match send_tx(
		&rest_client,
		chain_id,
		&deployer,
		module_address,
		"resource_roulette",
		"spin",
		vec![],
		vec![],
	)
	.await
	{
		Ok(tx6) => println!("Spin with Deployer: {:?}", tx6),
		Err(e) => {
			println!("Transaction failed: {:?}", e);
		}
	}

	// let multisig_account = send_tx(
	// 	&rest_client,
	// 	chain_id,
	// 	&alice,
	// 	"0x1",
	// 	"multisig_account",
	// 	"create_with_owners",
	// 	empty_type_tag.clone(),
	// 	vec![
	// 		bcs::to_bytes(&vec![bob.address()]).unwrap(),
	// 		bcs::to_bytes(&(2 as u8)).unwrap(),
	// 		vec![],
	// 		vec![],
	// 	],)
	// .await?;

	// send_tx(
	// 	&rest_client,
	// 	chain_id,
	// 	&alice,
	// 	"0x1",
	// 	"aptos_account",
	// 	"transfer",
	// 	empty_type_tag.clone(),
	// 	&vec![MoveValue::Address(multisig_account.clone()), MoveValue::U64(1)]
	// 	).await?;

	

	Ok(())
}


#[test]
fn hey_partners_load() {
	let config = ExecutionConfig::default();
	if let Err(err) = init_test(&config) {
		println!("Hey Partners Load Test init fail {err}",);
	}

	let result = execute_test(config, Arc::new(create_hey_partners_scenario));
	tracing::info!("Hey Partners Load Test result: {:?}", result);
}

#[test]
fn hey_partners_soak() {
	let mut config = ExecutionConfig::default();
	config.kind = TestKind::Soak {
		min_scenarios: 1,
		max_scenarios: 1,
		duration: std::time::Duration::from_secs(60),
		number_cycle: 1,
	};
	if let Err(err) = init_test(&config) {
		println!("Hey Partners Soak Test init fail {err}",);
	}

	let result = execute_test(config, Arc::new(create_hey_partners_scenario));
	tracing::info!("Hey Partners Soak Test result: {:?}", result);
}


fn create_hey_partners_scenario(_id: usize) -> Box<dyn Scenario> {
	Box::new(HeyPartnersScenario)
}
struct HeyPartnersScenario;

#[async_trait::async_trait]
impl Scenario for HeyPartnersScenario {
	async fn run(self: Box<Self>) -> Result<(), anyhow::Error> {
		test_hey_partners_internal().await
	}
}

#[tokio::test]
pub async fn test_hey_partners() -> Result<(), anyhow::Error> {
	test_hey_partners_internal().await
}

async fn test_hey_partners_internal() -> Result<(), anyhow::Error> {
    let root: PathBuf = cargo_workspace()?;
	let additional_path = "networks/suzuka/suzuka-client/src/tests/hey-partners/";
	let combined_path = root.join(additional_path);

	let test = combined_path.to_string_lossy();
	println!("{}", test);

    let output =
		run_command("/bin/bash", &[format!("{}{}", test, "test.sh").as_str()]).await?;
    println!("Output: {}", output);
    Ok(())
}
