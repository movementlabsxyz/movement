use anyhow::Context;
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
// <:!:section_1c

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect(
		"CARGO_MANIFEST_DIR is not set. Make sure to run this inside a Cargo build context.",
	);

	let target_dir =
		PathBuf::from(manifest_dir).join("networks/movement/movement-client/src/move_modules");

	// Run the `movement move build` command
	let build_status = Command::new("movement")
		.arg("move")
		.arg("build")
		.current_dir(target_dir)
		.status()
		.await
		.expect("Failed to execute `movement move build` command");

	// Check if the build succeeded
	if !build_status.success() {
		anyhow::bail!(
			"Building Move module failed. Please check the `movement move build` command."
		);
	}

	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

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
