use anyhow::Context;
use movement_client::{
	coin_client::CoinClient, crypto::ed25519::Ed25519PrivateKey, rest_client::Client,
	types::account_address::AccountAddress, types::account_config::aptos_test_root_address,
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
		format!("https://{}:{}", node_connection_address, node_connection_port);

	Url::from_str(node_connection_url.as_str()).unwrap()
});

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// :!:>section_1a
	let rest_client = Client::new(NODE_URL.clone());

	// :!:>section_1b
	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

	let raw_private_key = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.chain
		.maptos_private_key_signer_identifier
		.try_raw_private_key()?;
	let private_key = Ed25519PrivateKey::try_from(raw_private_key.as_slice())?;
	let mut genesis = LocalAccount::new(aptos_test_root_address(), private_key, 13007);
	let target_address = AccountAddress::from_hex_literal(
		"0x55f97e3f24410c4f3874c469b525c4076aaf02b8fee3c604a349a9fd9c947bc0",
	)?;

	// Have Alice send Bob some coins.
	let txn_hash = coin_client
		.transfer(&mut genesis, target_address, 1_000, None)
		.await
		.context("Failed to submit transaction to transfer coins")?;
	println!("Transaction submitted with hash: {:?}", txn_hash);
	rest_client
		.wait_for_transaction(&txn_hash)
		.await
		.context("Failed when waiting for the transfer transaction")?;

	Ok(())
}
