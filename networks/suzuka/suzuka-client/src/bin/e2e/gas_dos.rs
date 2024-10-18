use anyhow::Context;
use bcs::to_bytes;
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use suzuka_client::{
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
use url::Url;

static SUZUKA_CONFIG: Lazy<suzuka_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>().unwrap();
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

	// TEST 1: Sending a transaction trying to transfer more coins than Alice has (including gas fees)
	let transaction = create_fake_signed_transaction(
		chain_id,
		&alice,
		bob.address(),
		100_000_000,
		alice.sequence_number(),
	)
	.await?;

	let _transaction_will_fail = rest_client
		.submit(&transaction)
		.await
		.context("Failed when waiting for the transaction")?
		.into_inner();
	match rest_client.wait_for_signed_transaction(&transaction).await {
		Ok(_) => panic!("Transaction should have failed"),
		Err(e) => {
			println!("Transaction failed as expected: {:?}", e);
		}
	}

	// assert gas fee charged
	let failed_balance = coin_client
		.get_account_balance(&alice.address())
		.await
		.context("Failed to get Alice's account balance")?;
	println!("\n=== After Failed Tx#1 ===");
	println!("Alice: {:?}", failed_balance);
	assert!(initial_balance > failed_balance);

	// TEST 2: Sending a transaction with a high sequence number
	let too_high_sequence_number = alice.sequence_number() + 32 + 2;
	println!("Alice's sequence number: {}", alice.sequence_number());
	println!("Too high sequence number: {}", too_high_sequence_number);
	let mut last_balance = failed_balance;
	let transaction = create_fake_signed_transaction(
		chain_id,
		&alice,
		bob.address(),
		100,
		too_high_sequence_number, // too new tolerance is 32
	)
	.await?;

	match rest_client.submit(&transaction).await {
		Ok(_) => panic!("Transaction should have failed with high sequence number"),
		Err(e) => match e {
			suzuka_client::rest_client::error::RestError::Api(aptos_error) => {
				println!("Transaction failed as expected: {:?}", aptos_error);
				assert_eq!(aptos_error.error.error_code as u32, 402); // 402 is used for too old and too new
			}
			_ => panic!("Unexpected error: {:?}", e),
		},
	}

	// assert that no gas fee charged because the transaction never entered the mempool
	let failed_balance = coin_client
		.get_account_balance(&alice.address())
		.await
		.context("Failed to get Alice's account balance")?;
	println!("\n=== After Failed Tx#2 ===");
	println!("Alice: {:?}", failed_balance);
	assert!(last_balance == failed_balance);

	// TEST 3: Sending a transaction with a sequence number that won't be accepted by the VM, but would be accepted by the mempool (sequence number cannot be reused)
	let attack_sequence_number = alice.sequence_number() + 5;
	let transaction = create_fake_signed_transaction(
		chain_id,
		&alice,
		bob.address(),
		100,
		attack_sequence_number,
	)
	.await?;

	// transaction should fail in the vm not on the submission
	let _transaction_will_fail = rest_client
		.submit(&transaction)
		.await
		.context("Failed when waiting for the transaction")?
		.into_inner();
	match rest_client.wait_for_signed_transaction(&transaction).await {
		Ok(_) => panic!("Transaction should have failed"),
		Err(e) => {
			println!("Transaction failed as expected: {:?}", e);
		}
	}

	// assert gas fee not charged
	let failed_balance = coin_client
		.get_account_balance(&alice.address())
		.await
		.context("Failed to get Alice's account balance")?;
	println!("\n=== After Failed Tx#3 ===");
	println!("Alice: {:?}", failed_balance);
	assert!(last_balance == failed_balance);

	// transaction using the same sequence number should fail to submit
	let transaction = create_fake_signed_transaction(
		chain_id,
		&alice,
		bob.address(),
		100,
		attack_sequence_number,
	)
	.await?;

	match rest_client.submit(&transaction).await {
		Ok(res) => panic!(
			"Transaction should have failed with high sequence number. Instead got: {:?}",
			res
		),
		Err(e) => match e {
			suzuka_client::rest_client::error::RestError::Api(aptos_error) => {
				println!("Transaction failed as expected: {:?}", aptos_error);
				assert_eq!(aptos_error.error.error_code as u32, 402); // 402 is used for too old and too new
			}
			_ => panic!("Unexpected error: {:?}", e),
		},
	}

	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	test_sending_failed_transaction().await?;
	Ok(())
}
