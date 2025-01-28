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

const NUM_ACCOUNTS: usize = 5;
const TRANSACTIONS_PER_ACCOUNT: usize = 100;
const INITIAL_FUNDING: u64 = 10_000_000;
const TRANSFER_AMOUNT: u64 = 100;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	let ggp_address = get_governed_gas_pool_address(&rest_client).await?;

	let mut accounts = create_and_fund_accounts(&faucet_client, NUM_ACCOUNTS).await?;

	let initial_pool_balance = coin_client
		.get_account_balance(&ggp_address)
		.await
		.context("Failed to get initial gas pool balance")?;
	tracing::info!("Initial gas pool balance: {}", initial_pool_balance);

	execute_transaction_rounds(&mut accounts, &coin_client, &rest_client).await?;

	let final_pool_balance = coin_client
		.get_account_balance(&ggp_address)
		.await
		.context("Failed to get final gas pool balance")?;
	tracing::info!("Final gas pool balance: {}", final_pool_balance);

	assert!(
		final_pool_balance > initial_pool_balance,
		"Gas pool balance did not increase after {} transactions",
		NUM_ACCOUNTS * TRANSACTIONS_PER_ACCOUNT
	);

	tracing::info!("Total gas fees collected: {}", final_pool_balance - initial_pool_balance);

	Ok(())
}

async fn get_governed_gas_pool_address(
	rest_client: &Client,
) -> Result<AccountAddress, anyhow::Error> {
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

	Ok(AccountAddress::from_str(&ggp_address[0]).expect("Failed to parse address"))
}

async fn create_and_fund_accounts(
	faucet_client: &FaucetClient,
	num_accounts: usize,
) -> Result<Vec<LocalAccount>, anyhow::Error> {
	let mut accounts = Vec::with_capacity(num_accounts);

	for i in 0..num_accounts {
		let account = LocalAccount::generate(&mut rand::rngs::OsRng);
		tracing::info!("Creating account {}: {}", i, account.address());

		faucet_client
			.fund(account.address(), INITIAL_FUNDING)
			.await
			.context(format!("Failed to fund account {}", i))?;

		accounts.push(account);
	}

	Ok(accounts)
}

async fn execute_transaction_rounds(
	accounts: &mut [LocalAccount],
	coin_client: &CoinClient,
	rest_client: &Client,
) -> Result<(), anyhow::Error> {
	for round in 0..TRANSACTIONS_PER_ACCOUNT {
		tracing::info!("Starting transaction round {}", round);

		// Each account sends a transaction to the next account in the list
		for i in 0..accounts.len() {
			let sender_idx = i;
			let receiver_idx = (i + 1) % accounts.len();

			let txn_hash = coin_client
				.transfer(
					&mut accounts[sender_idx],
					accounts[receiver_idx].address(),
					TRANSFER_AMOUNT,
					None,
				)
				.await
				.context(format!(
					"Failed to submit transfer from account {} to {}",
					sender_idx, receiver_idx
				))?;

			rest_client
				.wait_for_transaction(&txn_hash)
				.await
				.context("Failed when waiting for transfer transaction")?;

			if round % 10 == 0 && i == 0 {
				tracing::info!("Completed {} transactions per account", round + 1);
			}
		}
	}

	Ok(())
}
