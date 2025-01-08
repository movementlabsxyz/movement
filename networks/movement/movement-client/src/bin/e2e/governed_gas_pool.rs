use anyhow::Context;
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

	/// Gas pool account
	let framework_address = "0x1";

	/// Create account for transactions and gas collection
	let mut sender = LocalAccount::generate(&mut rand::rngs::OsRng);
	let beneficiary = LocalAccount::generate(&mut rand::rngs::OsRng);

	tracing::info!("Created test accounts");
	tracing::debug!(
		"Sender address: {}, Beneficiary address: {}",
		sender.address(),
		beneficiary.address()
	);

	/// Fund the sender account
	faucet_client
		.fund(sender.address(), 1_000_000)
		.await
		.context("Failed to fund sender account")?;

	/// Create the beneficiary account
	faucet_client
		.create_account(beneficiary.address())
		.await
		.context("Failed to create beneficiary account")?;

	/// Get initial balances
	let initial_framework_balance = coin_client
		.get_account_balance(&framework_address.parse()?)
		.await
		.context("Failed to get initial framework balance")?;

	tracing::info!("Initial Framework Account Balance: {}", initial_framework_balance);

	/// Simple transaction that will generate gas fees
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

	/// Get post-transaction balance
	let post_txn_framework_balance = coin_client
		.get_account_balance(&framework_address.parse()?)
		.await
		.context("Failed to get post-transaction framework balance")?;

	tracing::info!("Post-Transaction Framework Balance: {}", post_txn_framework_balance);

	/// Verify gas fees collection
	assert!(
		post_txn_framework_balance > initial_framework_balance,
		"Gas fees were not collected as expected"
	);

	tracing::info!("Waiting to verify no additional deposits...");
	/// Wait to verify no additional deposits
	tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

	/// Check final balance
	let final_framework_balance = coin_client
		.get_account_balance(&framework_address.parse()?)
		.await
		.context("Failed to get final framework balance")?;

	/// Verify no additional deposits occurred
	assert_eq!(
		post_txn_framework_balance, final_framework_balance,
		"Additional unexpected deposits were detected"
	);

	tracing::info!("Test completed successfully");
	tracing::info!("=== Test Results ===");
	tracing::info!("Initial balance: {}", initial_framework_balance);
	tracing::info!("Final balance: {}", final_framework_balance);
	tracing::info!("Gas fees collected: {}", final_framework_balance - initial_framework_balance);

	Ok(())
}
