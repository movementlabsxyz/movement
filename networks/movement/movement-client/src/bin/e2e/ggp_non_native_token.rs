#![allow(unused_imports)]
#![allow(dead_code)]
use anyhow::Context;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::rest_client::Transaction;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::{move_types::language_storage::TypeTag, transaction_builder::TransactionFactory};
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use movement_client::crypto::ValidCryptoMaterialStringExt;
use movement_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use once_cell::sync::Lazy;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::process::Command;
use url::Url;

/// limit of gas unit
const GAS_UNIT_LIMIT: u64 = 100000;
/// minimum price of gas unit of aptos chains
pub const GAS_UNIT_PRICE: u64 = 100;
const ACCOUNT_ADDRESS: &str = "30005dbbb9b324b18bed15aca87770512ec7807410fabb0420494d9865e56fa4";

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
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

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	let mut core_resources_account = LocalAccount::from_private_key(
		SUZUKA_CONFIG
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key
			.to_encoded_string()?
			.as_str(),
		0,
	)?;

	let crate_dir = env::var("CARGO_MANIFEST_DIR").expect(
		"CARGO_MANIFEST_DIR is not set. Make sure to run this inside a Cargo build context.",
	);

	let target_dir = PathBuf::from(crate_dir).join("src").join("move-modules");

	println!("target_dir: {:?}", target_dir);
	println!("Node URL: {:?}", NODE_URL.as_str());
	println!("Faucet URL: {:?}", FAUCET_URL.as_str());

	//check current_dir println
	println!("current_dir: {:?}", env::current_dir().unwrap());

	let publish_status = Command::new("movement")
		.args([
			"move",
			"publish",
			"--package-dir",
			"networks/movement/movement-client/src/move-modules",
			"--skip-fetch-latest-git-deps",
			"--sender-account",
			ACCOUNT_ADDRESS,
			"--assume-yes",
		])
		.status()
		.await
		.expect("Failed to execute `movement move publish` command");

	// Check if the publish succeeded
	if !publish_status.success() {
		anyhow::bail!(
			"Publishing Move module failed. Please check the `movement move publish` command."
		);
	}

	//Validate it is published at the expected address
	let _ = Command::new("movement")
		.args(["account", "list", "--account", ACCOUNT_ADDRESS])
		.status()
		.await
		.expect("Failed to execute `movement move resource` command");

	let init_payload = make_entry_function_payload(
		//NB: package address arg, this is the account address of the sender
		AccountAddress::from_hex_literal(&format!("0x{}", ACCOUNT_ADDRESS)).unwrap(),
		"GGPTestToken",
		"initialize_test_token",
		vec![],
		vec![],
	);

	let deposit_status = Command::new("movement")
		.args(&[
			"move",
			"run-script",
			"--compiled-script-path",
			"build/GGPTestToken/bytecode_scripts/main.mv",
			"--args",
			"u64: 42",
			"--profile",
			"default",
			"--assume-yes",
			"--sender-account",
			"0xa550c18",
		])
		.status()
		.await
		.expect("Failed to execute `movement move run-script` command");

	println!("deposit_status: {:?}", deposit_status);

	if !deposit_status.success() {
		anyhow::bail!("Deposit failed. Please check the `movement move run-script` command.");
	}

	//If you don't remove .movement/ between runs this seq number will be wrong
	// let mut sequence_number = 1;
	// let signer = &mut LocalAccount::from_private_key(PRIVATE_KEY, sequence_number)?;
	//
	// println!("sending initialize_test_token tx with payload {:?}", init_payload);
	//
	// let tx_response = send_aptos_transaction(&rest_client, signer, init_payload).await?;
	//
	// println!("Transaction response: {:?}", tx_response);
	//
	// let deposit_script = fs::read(
	// 	std::env::current_dir()?
	// 		.join("build")
	// 		.join("GGPTestToken")
	// 		.join("bytecode_scripts")
	// 		.join("main.mv"),
	// )?;
	//
	// // let args = vec![TransactionArgument::U64(42)];
	// // let script_payload = TransactionPayload::Script(Script::new(code, vec![], args));
	//
	// let mut harness = MoveHarness::new_testnet();
	//
	// let core_resources =
	// 	harness.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());
	//
	// let tx = harness.create_script(
	// 	&core_resources,
	// 	deposit_script,
	// 	vec![],
	// 	vec![TransactionArgument::U64(42)],
	// );
	//
	// println!("TX: {:?}", tx);
	//
	// let tx_status = harness.run(tx);
	//
	// println!("tx_status: {:?}", tx_status);
	//
	// let state = rest_client
	// 	.get_ledger_information()
	// 	.await
	// 	.context("Failed in getting chain id")?
	// 	.into_inner();

	//let tx_builder = core_resources.transaction();

	// let signed_tx = tx_builder
	// 	.chain_id(ChainId::new(state.chain_id))
	// 	.sequence_number(0)
	// 	.ttl(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60)
	// 	.gas_unit_price(GAS_UNIT_PRICE)
	// 	.payload(script_payload.clone())
	// 	.sign();
	//
	// println!("signed_tx: {:?}", signed_tx);
	//
	// let response = rest_client.submit_and_wait(&signed_tx).await?;
	//
	// println!("tx_response: {:?}", response);
	//
	Ok(())
}

async fn send_aptos_transaction(
	client: &Client,
	signer: &mut LocalAccount,
	payload: TransactionPayload,
) -> anyhow::Result<Transaction> {
	let state = client
		.get_ledger_information()
		.await
		.context("Failed in getting chain id")?
		.into_inner();

	let transaction_factory = TransactionFactory::new(ChainId::new(state.chain_id))
		.with_gas_unit_price(100)
		.with_max_gas_amount(GAS_UNIT_LIMIT);

	let signed_tx = signer.sign_with_transaction_builder(transaction_factory.payload(payload));

	let response = client
		.submit_and_wait(&signed_tx)
		.await
		.map_err(|e| anyhow::anyhow!(e.to_string()))?
		.into_inner();
	Ok(response)
}

fn make_entry_function_payload(
	package_address: AccountAddress,
	module_name: &'static str,
	function_name: &'static str,
	ty_args: Vec<TypeTag>,
	args: Vec<Vec<u8>>,
) -> TransactionPayload {
	println!("package_address: {:?}", package_address);
	TransactionPayload::EntryFunction(EntryFunction::new(
		ModuleId::new(package_address, Identifier::new(module_name).unwrap()),
		Identifier::new(function_name).unwrap(),
		ty_args,
		args,
	))
}
