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

pub async fn create_fake_signed_transaction(
	chain_id: u8,
	from_account: &LocalAccount,
	to_account: AccountAddress,
	amount: u64,
	sequence_number: u64,
) -> Result<SignedTransaction, anyhow::Error> {
	let coin_type = "0x1::aptos_coin::AptosCoin";
	let timeout_secs = 30; // 30 seconds
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

pub async fn test_sending_failed_transaction() -> Result<(), anyhow::Error> {
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	let alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng);

	println!("=== Addresses ===");
	println!("Alice: {}", alice.address().to_hex_literal());
	println!("Bob: {}", bob.address().to_hex_literal());

	faucet_client
		.fund(alice.address(), 100_000_000)
		.await
		.context("Failed to fund Alice's account")?;

	faucet_client
		.create_account(bob.address())
		.await
		.context("Failed to fund Bob's account")?;

	let chain_id = rest_client
		.get_index()
		.await
		.context("Failed to get chain ID")?
		.inner()
		.chain_id;
	println!("\n=== Initial Balance ===");
	let initial_balance = coin_client
		.get_account_balance(&alice.address())
		.await
		.context("Failed to get Alice's account balance")?;
	println!("Alice: {:?}", initial_balance);

	// Send in ooo sequence numbers
	// create transaction 1
	let transaction_1 = create_fake_signed_transaction(
		chain_id,
		&alice,
		bob.address(),
		100,
		alice.sequence_number(),
	)
	.await?;

	// create transaction 2
	let transaction_2 = create_fake_signed_transaction(
		chain_id,
		&alice,
		bob.address(),
		100,
		alice.sequence_number() + 1,
	)
	.await?;

	// submit 2 then 1
	rest_client.submit(&transaction_2).await?;
	rest_client.submit(&transaction_1).await?;

	tracing::info!("=== Transactions Submitted ===");
	println!("Tx#1: {:?}", transaction_1);
	println!("Tx#2: {:?}", transaction_2);

	// wait for both the complete
	tracing::info!("=== Waiting for transactions to be executed ===");
	let res_tx2 = rest_client.wait_for_signed_transaction(&transaction_2).await?;
	let res_tx1 = rest_client.wait_for_signed_transaction(&transaction_1).await?;

	// validate the effect of the transactions, the balance should be less than the initial balance
	let balance = coin_client
		.get_account_balance(&alice.address())
		.await
		.context("Failed to get Alice's account balance")?;
	tracing::info!("\n=== After Tx#1 and Tx#2 ===");
	println!("Alice: {:?}", balance);
	assert!(initial_balance > balance);

	let balance = coin_client
		.get_account_balance(&bob.address())
		.await
		.context("Failed to get Alice's account balance")?;
	println!("Bob: {:?}", balance);
	println!("res_tx1: {:?}", res_tx1);
	println!("res_tx2: {:?}", res_tx2);

	let alice = rest_client.get_account(alice.address()).await?.into_inner();
	let bob = rest_client.get_account(bob.address()).await?.into_inner();

	println!("Alice seq: {:?}", alice.sequence_number);
	println!("Bob seq: {:?}", bob.sequence_number);

	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	test_sending_failed_transaction().await?;
	Ok(())
}
