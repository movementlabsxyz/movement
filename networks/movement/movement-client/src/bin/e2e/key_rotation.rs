use anyhow::Context;
use aptos_sdk::rest_client::{
	aptos_api_types::{Address, EntryFunctionId, IdentifierWrapper, MoveModuleId, ViewRequest},
	Account, Response,
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

	// Create account for transactions and gas collection
	let mut sender = LocalAccount::generate(&mut rand::rngs::OsRng);
	let beneficiary = LocalAccount::generate(&mut rand::rngs::OsRng);

	let acc = AccountAddress::from_str(
		"0xb08e0478ac871400e082f34e003145570bf4a9e4d88f17964b21fb110e93d77a",
	)
	.unwrap();

	tracing::info!("Created test accounts");
	tracing::debug!(
		"Sender address: {}, Beneficiary address: {}",
		sender.address(),
		beneficiary.address()
	);

	// Fund the sender account
	faucet_client
		.fund(sender.address(), 1_000_000)
		.await
		.context("Failed to fund sender account")?;

	// Create the beneficiary account
	faucet_client
		.create_account(beneficiary.address())
		.await
		.context("Failed to create beneficiary account")?;

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

	// Extract the inner field from the response
	let inner_value = serde_json::to_value(view_res.inner())
		.context("Failed to convert response inner to serde_json::Value")?;

	// Deserialize the inner value into your AddressResponse struct
	let ggp_address: Vec<String> =
		serde_json::from_value(inner_value).context("Failed to deserialize AddressResponse")?;

	assert_eq!(
		ggp_address,
		vec!["0xb08e0478ac871400e082f34e003145570bf4a9e4d88f17964b21fb110e93d77a"],
		"Governed Gas Pool Resource account is not what is expected"
	);

	let ggp_account_address =
		AccountAddress::from_str(&ggp_address[0]).expect("Failed to parse address");

	// Get initial balances
	let initial_ggp_balance = coin_client
		.get_account_balance(&ggp_account_address)
		.await
		.context("Failed to get initial framework balance")?;

	tracing::info!("Initial ggp Balance: {}", initial_ggp_balance);

	// Simple transaction that will generate gas fees
	tracing::info!("Executing test transaction...");
	let txn_hash = coin_client
		.transfer(&mut sender, beneficiary.address(), 1_000, None)
		.await
		.context("Failed to submit transfer transaction")?;

	rest_client
		.wait_for_transaction(&txn_hash)
		.await
		.context("Failed when waiting for transfer transaction")?;
	tracing::info!("Test transaction completed: {:?}", txn_hash);

	// Get post-transaction balance
	let post_ggp_balance = coin_client
		.get_account_balance(&ggp_account_address)
		.await
		.context("Failed to get post-transaction framework balance")?;

	tracing::info!("Initial ggp Balance: {}", initial_ggp_balance);

	// Verify gas fees collection
	assert!(post_ggp_balance > initial_ggp_balance, "Gas fees were not collected as expected");

	// Wait to verify no additional deposits
	tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

	// Check final balance
	let final_framework_balance = coin_client
		.get_account_balance(&ggp_account_address)
		.await
		.context("Failed to get final framework balance")?;

	// Verify no additional deposits occurred
	assert_eq!(
		post_ggp_balance, final_framework_balance,
		"Additional unexpected deposits were detected"
	);

	Ok(())
}
