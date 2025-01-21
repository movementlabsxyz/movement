use anyhow::Context;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::rest_client::Transaction;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::{move_types::language_storage::TypeTag, transaction_builder::TransactionFactory};
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::{EntryFunction, Script, TransactionArgument, TransactionPayload};
use movement_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use once_cell::sync::Lazy;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::process::Command;
use url::Url;

/// limit of gas unit
const GAS_UNIT_LIMIT: u64 = 100000;
/// minimum price of gas unit of aptos chains
pub const GAS_UNIT_PRICE: u64 = 100;
const ACCOUNT_ADDRESS: &str = "30005dbbb9b324b18bed15aca87770512ec7807410fabb0420494d9865e56fa4";
//
// NB: This is a determinisitic privake key generated from the init command
// used for testing purposes only
const PRIVATE_KEY: &str = "0x97121e4f94695b6fb65a24899c5cce23cc0dad5a1c07caaeb6dd555078d14ba7";

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

	let crate_dir = env::var("CARGO_MANIFEST_DIR").expect(
		"CARGO_MANIFEST_DIR is not set. Make sure to run this inside a Cargo build context.",
	);
	let crate_dir_clone = crate_dir.clone();

	let target_dir = PathBuf::from(crate_dir).join("src").join("move-modules");
	let target_dir_clone = target_dir.clone();

	println!("target_dir: {:?}", target_dir);
	println!("Node URL: {:?}", NODE_URL.as_str());
	println!("Faucet URL: {:?}", FAUCET_URL.as_str());

	// Use the account value generated from the init command
	// found in ./movement/config.yaml
	let init_status = Command::new("movement")
		.args([
			"init",
			"--network",
			"custom",
			"--rest-url",
			NODE_URL.as_str(),
			"--faucet-url",
			FAUCET_URL.as_str(),
			"--assume-yes",
			"--private-key",
			PRIVATE_KEY,
		])
		.current_dir(target_dir) // think this is wrong .. check
		.status()
		.await
		.expect("Failed to execute `movement init` command");

	if !init_status.success() {
		anyhow::bail!("Initializing Move module failed. Please check the `movement init` command.");
	}

	env::set_current_dir(&target_dir_clone).expect("Failed to set current directory");

	//check current_dir println
	println!("current_dir: {:?}", env::current_dir().unwrap());

	// account associated with private key used for init
	let publish_status = Command::new("movement")
		.args([
			"move",
			"publish",
			"--skip-fetch-latest-git-deps",
			"--sender-account",
			ACCOUNT_ADDRESS,
			"--assume-yes",
		])
		.status()
		.await
		.expect("Failed to execute `movement move publish` command");

	println!("publish_status: {:?}", publish_status);

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

	//If you don't remove .movement/ between runs this value will increment
	//by one and you'll get an error.
	let sequence_number = 1;

	let tx_response = send_aptos_transaction(
		&rest_client,
		&mut LocalAccount::from_private_key(PRIVATE_KEY, sequence_number)?,
		init_payload,
	)
	.await?;

	println!("Transaction response: {:?}", tx_response);

	let crate_dir = PathBuf::from(crate_dir_clone);

	let code = fs::read(
		crate_dir
			.join("src")
			.join("move-modules")
			.join("build")
			.join("bytecode_scripts")
			.join("main.mv"),
	)?;
	let args = vec![TransactionArgument::U64(42)];

	let script_payload = TransactionPayload::Script(Script::new(code, vec![], args));

	let tx_response = send_aptos_transaction(
		&rest_client,
		&mut LocalAccount::from_private_key(PRIVATE_KEY, sequence_number)?,
		script_payload,
	)
	.await?;

	println!("tx_response: {:?}", tx_response);

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
