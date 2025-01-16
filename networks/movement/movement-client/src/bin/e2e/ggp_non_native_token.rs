use anyhow::Context;
use aptos_sdk::move_types::ident_str;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::rest_client::Transaction;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::{move_types::language_storage::TypeTag, transaction_builder::TransactionFactory};
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::{EntryFunction, TransactionPayload};
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
		.status()
		.await
		.expect("Failed to execute `movement init` command");

	if !init_status.success() {
		anyhow::bail!("Initializing Move module failed. Please check the `movement init` command.");
	}

	let target_dir = PathBuf::from(crate_dir).join("src").join("move-modules");
	let target_dir_clone = target_dir.clone();

	println!("target_dir: {:?}", target_dir);

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
		.current_dir(target_dir_clone)
		.status()
		.await
		.expect("Failed to execute `movement move publish` command");

	// Check if the publish succeeded
	if !publish_status.success() {
		anyhow::bail!(
			"Publishing Move module failed. Please check the `movement move publish` command."
		);
	}

	let args = vec![bcs::to_bytes(
		&AccountAddress::from_hex_literal(&format!(
			"0x{}",
			hex::encode(&AccountAddress::from_str(ACCOUNT_ADDRESS).unwrap().to_vec())
		))
		.unwrap(),
	)
	.unwrap()];

	let init_payload = make_aptos_payload(
		AccountAddress::from_str(
			"0x97121e4f94695b6fb65a24899c5cce23cc0dad5a1c07caaeb6dd555078d14ba7",
		)
		.unwrap(),
		"test_token",
		"initialize_test_token",
		vec![],
		args,
	);

	let tx_response = send_aptos_transaction(
		&rest_client,
		&mut LocalAccount::from_private_key(PRIVATE_KEY, 0)?,
		init_payload,
	)
	.await?;

	// Create the proposer account and fund it from the faucet
	let proposer = LocalAccount::generate(&mut rand::rngs::OsRng);
	faucet_client
		.fund(proposer.address(), 1_000_000)
		.await
		.context("Failed to fund proposer account")?;

	// Create the beneficiary account and fund it from the faucet
	let beneficiary = LocalAccount::generate(&mut rand::rngs::OsRng);
	faucet_client
		.fund(beneficiary.address(), 1_000_000)
		.await
		.context("Failed to fund beneficiary account")?;
	let beneficiary_address = beneficiary.address().to_hex_literal();

	// TODO: run some methods, collect some gas, and check the balance of the governed gas pool

	let amount = 100_000; // TODO: replace with appropriate amount w.r.t. gas collection.

	let pre_beneficiary_balance = coin_client
		.get_account_balance(&beneficiary.address())
		.await
		.context("Failed to get beneficiary's account balance")?;

	Ok(())
}

#[allow(dead_code)]
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

fn make_aptos_payload(
	package_address: AccountAddress,
	module_name: &'static str,
	function_name: &'static str,
	ty_args: Vec<TypeTag>,
	args: Vec<Vec<u8>>,
) -> TransactionPayload {
	TransactionPayload::EntryFunction(EntryFunction::new(
		ModuleId::new(package_address, ident_str!(module_name).to_owned()),
		ident_str!(function_name).to_owned(),
		ty_args,
		args,
	))
}
