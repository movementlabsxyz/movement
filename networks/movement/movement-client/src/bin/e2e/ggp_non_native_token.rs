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
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	let crate_dir = env::var("CARGO_MANIFEST_DIR").expect(
		"CARGO_MANIFEST_DIR is not set. Make sure to run this inside a Cargo build context.",
	);

	println!("Node URL: {:?}", NODE_URL.as_str());
	println!("Faucet URL: {:?}", FAUCET_URL.as_str());

	// let init_status = Command::new("movement")
	// 	.args([
	// 		"init",
	// 		"--network",
	// 		"custom",
	// 		"--rest-url",
	// 		NODE_URL.as_str(),
	// 		"--faucet-url",
	// 		FAUCET_URL.as_str(),
	// 		"--assume-no",
	// 	])
	// 	.status()
	// 	.await
	// 	.expect("Failed to execute `movement init` command");
	//
	// if !init_status.success() {
	// 	anyhow::bail!("Initializing Move module failed. Please check the `movement init` command.");
	// }

	//println!("init status: {:?}", init_status);

	let target_dir = PathBuf::from(crate_dir).join("src").join("move-modules");
	let target_dir_clone = target_dir.clone();

	println!("target_dir: {:?}", target_dir);

	let publish_status = Command::new("movement")
		.args(["move", "publish", "--skip-fetch-latest-git-deps"])
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

	println!("Move module build");

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
