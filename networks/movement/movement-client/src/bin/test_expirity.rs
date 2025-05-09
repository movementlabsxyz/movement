use anyhow::Context;
use bcs::to_bytes;
use movement_client::{
	coin_client::CoinClient,
	move_types::{
		identifier::Identifier,
		language_storage::{ModuleId, TypeTag},
	},
	rest_client::{Client, FaucetClient},
	transaction_builder::TransactionBuilder,
	types::transaction::{EntryFunction, SignedTransaction, TransactionPayload},
	types::{account_address::AccountAddress, chain_id::ChainId, LocalAccount},
};
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
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
	use tracing_subscriber::EnvFilter;
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();
	let client_url = NODE_URL.clone();
	let client_url = Url::from_str("https://testnet.bardock.movementnetwork.xyz/v1").unwrap();
	let rest_client = Client::new(client_url.clone());
	let faucet_url = FAUCET_URL.clone();
	let faucet_url = Url::from_str("https://faucet.testnet.bardock.movementnetwork.xyz/").unwrap();
	let faucet_client = FaucetClient::new(faucet_url, client_url);
	let faucet_client = faucet_client.with_auth_token("notreal".to_string());
	let coin_client = CoinClient::new(&rest_client);
	let chain_id = rest_client
		.get_index()
		.await
		.context("Failed to get chain ID")?
		.inner()
		.chain_id;

	// Create two accounts locally, Alice and Bob.
	let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

	// Print account addresses.
	tracing::info!(
		"Start Addresses ===\nAlice: {}\nBob: {}",
		alice.address().to_hex_literal(),
		bob.address().to_hex_literal()
	);

	// Create the accounts on chain, but only fund Alice.
	faucet_client.fund(alice.address(), 100_000_000_000).await?;
	faucet_client.create_account(bob.address()).await?;

	let transaction =
		create_signed_transaction(chain_id, &alice, bob.address(), 100, alice.sequence_number())
			.await?;
	tracing::info!("Tx res:{}", transaction.expiration_timestamp_secs());
	let tx_res = rest_client.submit(&transaction).await?.into_inner();
	tracing::info!("Tx res:{tx_res:?}");

	Ok(())
}

pub async fn create_signed_transaction(
	chain_id: u8,
	from_account: &LocalAccount,
	to_account: AccountAddress,
	amount: u64,
	sequence_number: u64,
) -> Result<SignedTransaction, anyhow::Error> {
	let coin_type = "0x1::aptos_coin::AptosCoin";
	let timeout_secs = 600; // 10 minutes
	let max_gas_amount = 5_000;
	let gas_unit_price = 100;

	let expiration_time =
		SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + timeout_secs;

	let transaction_builder = TransactionBuilder::new(
		TransactionPayload::EntryFunction(EntryFunction::new(
			ModuleId::new(AccountAddress::ONE, Identifier::new("coin").unwrap()),
			Identifier::new("transfer")?,
			vec![TypeTag::from_str(coin_type)?],
			vec![to_bytes(&to_account)?, to_bytes(&amount)?],
		)),
		expiration_time,
		ChainId::new(chain_id),
	);

	let raw_transaction = transaction_builder
		.sender(from_account.address())
		.sequence_number(sequence_number)
		.max_gas_amount(max_gas_amount)
		.gas_unit_price(gas_unit_price)
		.expiration_timestamp_secs(expiration_time)
		.chain_id(ChainId::new(chain_id))
		.build();

	let signed_transaction = from_account.sign_transaction(raw_transaction);

	Ok(signed_transaction)
}
