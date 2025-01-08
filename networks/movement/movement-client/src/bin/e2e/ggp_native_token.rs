use anyhow::Context;
use movement_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use once_cell::sync::Lazy;
use std::str::FromStr;
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
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

	// Create the proposer account and fund it from the faucet
	let mut proposer = LocalAccount::generate(&mut rand::rngs::OsRng);
	faucet_client
		.fund_account(&proposer, 1_000_000)
		.await
		.context("Failed to fund proposer account")?;

	// Create the beneficiary account and fund it from the faucet
	let beneficiary = LocalAccount::generate(&mut rand::rngs::OsRng);
	faucet_client
		.fund_account(&beneficiary, 1_000_000)
		.await
		.context("Failed to fund beneficiary account")?;
	let beneficiary_address = beneficiary.address().to_hex_literal();

	// TODO: run some methods, collect some gas, and check the balance of the governed gas pool

	let amount = 100_000; // TODO: replace with appropriate amount w.r.t. gas collection.

	let pre_beneficiary_balance = coin_client
		.get_account_balance(&beneficiary.address())
		.await
		.context("Failed to get beneficiary's account balance")?;

	// Build the move script payload
	let fund_from_governed_gas_pool_script = format!(
		r#"
script {{
	use aptos_framework::aptos_governance;
	use aptos_framework::consensus_config;
	use aptos_framework::governed_gas_pool;
	fun main(core_resources: &signer) {{
		let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
		// fund the beneficiary account
		governed_gas_pool::fund(
			&framework_signer,
			@0x{beneficiary_address},
			{amount}
		);
		
	}}
}}
"#
	);

	// Assert the emission of the governed gas pool
	let post_beneficiary_balance = coin_client
		.get_account_balance(&beneficiary.address())
		.await
		.context("Failed to get beneficiary's account balance")?;
	assert_eq!(
		post_beneficiary_balance,
		pre_beneficiary_balance + amount,
		"Beneficiary's balance should be increased by {}",
		amount
	);

	Ok(())
}

