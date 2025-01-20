use anyhow::Context;
use aptos_sdk::rest_client::{
	aptos_api_types::{Address, EntryFunctionId, IdentifierWrapper, MoveModuleId, ViewRequest},
	Response,
};
use aptos_sdk::types::account_address::AccountAddress;
use movement_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
	types::LocalAccount,
};
use once_cell::sync::Lazy;
use std::str::FromStr;
use tracing;
use url::Url;

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

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

	// Create a gas payer account (to fund the pool) and beneficiary account
	let mut gas_payer = LocalAccount::generate(&mut rand::rngs::OsRng);
	let beneficiary = LocalAccount::generate(&mut rand::rngs::OsRng);

	tracing::info!("Created test accounts");
	tracing::debug!(
		"Gas payer address: {}, Beneficiary address: {}",
		gas_payer.address(),
		beneficiary.address()
	);

	// Fund the gas payer account
	faucet_client
		.fund(gas_payer.address(), 1_000_000)
		.await
		.context("Failed to fund gas payer account")?;

	// Create and register the beneficiary account for APT
	faucet_client
		.create_account(beneficiary.address())
		.await
		.context("Failed to create beneficiary account")?;

	// Get the governed gas pool address
	let view_req = ViewRequest {
		function: EntryFunctionId {
			module: MoveModuleId {
				address: Address::from_str("0x1").unwrap(),
				name: IdentifierWrapper::from_str("governed_gas_pool").unwrap(),
			},
			name: IdentifierWrapper::from_str("governed_gas_pool_address").unwrap(),
		},
		type_arguments: vec![],
		arguments: vec![],
	};

	let view_res: Response<Vec<serde_json::Value>> = rest_client
		.view(&view_req, None)
		.await
		.context("Failed to get governed gas pool address")?;

	let inner_value = serde_json::to_value(view_res.inner())
		.context("Failed to convert response inner to serde_json::Value")?;

	let ggp_address: Vec<String> =
		serde_json::from_value(inner_value).context("Failed to deserialize AddressResponse")?;

	let ggp_account_address =
		AccountAddress::from_str(&ggp_address[0]).expect("Failed to parse address");

	// Make a transaction to generate gas fees that will go to the pool
	let txn_hash = coin_client
		.transfer(&mut gas_payer, beneficiary.address(), 1_000, None)
		.await
		.context("Failed to submit transfer transaction")?;

	rest_client
		.wait_for_transaction(&txn_hash)
		.await
		.context("Failed when waiting for transfer transaction")?;

	// Get initial balances before fund distribution
	let initial_pool_balance = coin_client
		.get_account_balance(&ggp_account_address)
		.await
		.context("Failed to get initial gas pool balance")?;

	let initial_beneficiary_balance = coin_client
		.get_account_balance(&beneficiary.address())
		.await
		.context("Failed to get initial beneficiary balance")?;

	tracing::info!("Initial gas pool balance: {}", initial_pool_balance);
	tracing::info!("Initial beneficiary balance: {}", initial_beneficiary_balance);

	let final_pool_balance = coin_client
		.get_account_balance(&ggp_account_address)
		.await
		.context("Failed to get final gas pool balance")?;

	assert!(final_pool_balance > initial_pool_balance, "Gas fees were not collected by the pool");

	Ok(())
}
