use aptos_sdk::types::account_address::AccountAddress;
/// The key_rotation.rs test tests the basic functionality of key rotation works as expected.
/// It essnetially makes the calls that the CLI cmd `movement account rotate-key` abstracts away.
/// This test makes checks the CLI level and checks on the existence of resource pre and post
/// rotation. And demonstrates the correct approach on how to correctly migrate resources and
/// balances for an account prior to key rotation. Failure to rotate resources will render them
/// unnaccessable after a key has been rotated out.
use aptos_sdk::{coin_client::CoinClient, rest_client::Client};
use once_cell::sync::Lazy;
use std::str::FromStr;
use tracing_subscriber::EnvFilter;
use url::Url;

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap()
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let node_connection_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port =
		SUZUKA_CONFIG.execution_config.maptos_config.client.maptos_rest_connection_port;
	let node_connection_url =
		format!("http://{}:{}", node_connection_address, node_connection_port);
	Url::from_str(&node_connection_url).unwrap()
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
		.maptos_faucet_rest_connection_port;

	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);

	Url::from_str(faucet_listen_url.as_str()).unwrap()
});

const ACCOUNT_ADDRESS: &str = "0xd1126ce48bd65fb72190dbd9a6eaa65ba973f1e1664ac0cfba4db1d071fd0c36";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let rest_client = Client::new(NODE_URL.clone());
	let account = AccountAddress::from_hex_literal(ACCOUNT_ADDRESS)?;

	let res = rest_client.get_account_resources(account).await?;
	println!("{:#?}", res);
	Ok(())
}
