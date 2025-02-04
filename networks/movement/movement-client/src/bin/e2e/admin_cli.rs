use anyhow::Context;
use aptos_sdk::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
};
use movement_client::types::LocalAccount;
use once_cell::sync::Lazy;
use std::str::FromStr;
//use tokio::process::Command;
use tracing::info;
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

const DEAD_ADDRESS: &str = "000000000000000000000000000000000000000000000000000000000000dead";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Initialize clients
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	// Load core resource account
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
	info!(
		"Core Resources Account keypairs: {:?}, {:?}",
		core_resources_account.private_key(),
		core_resources_account.public_key()
	);
	info!("Core Resources Account address: {}", core_resources_account.address());

	// Fund the account
	faucet_client.fund(core_resources_account.address(), 100_000_000_000).await?;

	let state = rest_client
		.get_ledger_information()
		.await
		.context("Failed in getting chain id")?
		.into_inner();

	// Generate recipient account
	let recipient = LocalAccount::generate(&mut rand::rngs::OsRng);

	faucet_client.fund(recipient.address(), 100_000_000_000).await?;

	let recipient_bal = coin_client
		.get_account_balance(&recipient.address())
		.await
		.context("Failed to get recipient's account balance")?;

	let core_resource_bal = coin_client
		.get_account_balance(&core_resources_account.address())
		.await
		.context("Failed to get core resources account balance")?;

	info!("Recipient's balance: {:?}", recipient_bal);
	info!("Core Resources Account balance: {:?}", core_resource_bal);

	Ok(())
}

