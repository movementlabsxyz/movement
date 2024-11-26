use anyhow::Context;
use movement_client::crypto::ValidCryptoMaterialStringExt;
use movement_client::{
	coin_client::CoinClient,
	move_types::identifier::Identifier,
	move_types::language_storage::ModuleId,
	rest_client::{Client, FaucetClient},
	transaction_builder::TransactionBuilder,
	types::account_address::AccountAddress,
	types::transaction::{EntryFunction, TransactionPayload},
	types::LocalAccount,
	BatchWriteRequest, BlobWrite, MovementDaLightNodeClient,
};
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
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

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// :!:>section_1a
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

	// :!:>section_1b
	let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

	// Create two accounts locally, Alice and Bob.
	// :!:>section_2
	let mut genesis = LocalAccount::from_private_key(
		SUZUKA_CONFIG
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key
			.to_encoded_string()?
			.as_str(),
		0,
	)?;
	let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
	let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

	// Print account addresses.
	println!("\n=== Addresses ===");
	println!("Genesis: {}", genesis.address().to_hex_literal());
	println!("Alice: {}", alice.address().to_hex_literal());
	println!("Bob: {}", bob.address().to_hex_literal());

	// Create the accounts on chain, but only fund Alice.
	// :!:>section_3
	faucet_client
		.fund(genesis.address(), 100_000_000)
		.await
		.context("Failed to fund genesis account")?;
	faucet_client
		.fund(alice.address(), 100_000_000)
		.await
		.context("Failed to fund Alice's account")?;
	faucet_client
		.create_account(bob.address())
		.await
		.context("Failed to fund Bob's account")?; // <:!:section_3

	// Print initial balances.
	println!("\n=== Initial Balances ===");
	println!(
		"Genesis: {:?}",
		coin_client
			.get_account_balance(&genesis.address())
			.await
			.context("Failed to get genesis account balance")?
	);
	println!(
		"Alice: {:?}",
		coin_client
			.get_account_balance(&alice.address())
			.await
			.context("Failed to get Alice's account balance")?
	);
	println!(
		"Bob: {:?}",
		coin_client
			.get_account_balance(&bob.address())
			.await
			.context("Failed to get Bob's account balance")?
	);

	// Have genesis send Alice some coins.
	// This should succeed because genesis is whitelisted.
	let txn_hash = coin_client
		.transfer(&mut genesis, alice.address(), 1_000, None)
		.await
		.context("Failed to submit transaction to transfer coins from genesis account")?;
	rest_client
		.wait_for_transaction(&txn_hash)
		.await
		.context("Failed when waiting for the transfer transaction from genesis account")?;

	// Have Alice send Bob some coins.
	// This should be reject on ingress because Alice is not whitelisted.
	assert!(coin_client.transfer(&mut alice, bob.address(), 1_000, None).await.is_err());

	// construct the direct MovementDaLightNodeClient
	let light_node_connection_protocol = SUZUKA_CONFIG
		.celestia_da_light_node
		.celestia_da_light_node_config
		.movement_da_light_node_connection_protocol();

	// todo: extract into getter
	let light_node_connection_hostname = SUZUKA_CONFIG
		.celestia_da_light_node
		.celestia_da_light_node_config
		.movement_da_light_node_connection_hostname();

	// todo: extract into getter
	let light_node_connection_port = SUZUKA_CONFIG
		.celestia_da_light_node
		.celestia_da_light_node_config
		.movement_da_light_node_connection_port();

	let mut da_client = MovementDaLightNodeClient::try_http1(
		format!(
			"{}://{}:{}",
			light_node_connection_protocol,
			light_node_connection_hostname,
			light_node_connection_port
		)
		.as_str(),
	)?;

	// Create a raw transaction from Alice to Bob.
	let transaction_builder = TransactionBuilder::new(
		TransactionPayload::EntryFunction(EntryFunction::new(
			ModuleId::new(AccountAddress::from_str_strict("0x1")?, Identifier::new("coin")?),
			Identifier::new("transfer")?,
			vec![],
			vec![],
		)),
		SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 20,
		SUZUKA_CONFIG.execution_config.maptos_config.chain.maptos_chain_id.clone(),
	)
	.sender(alice.address())
	.sequence_number(alice.sequence_number())
	.max_gas_amount(5_000)
	.gas_unit_price(100);

	// create the blob write
	let signed_transaction = alice.sign_with_transaction_builder(transaction_builder);
	let txn_hash = signed_transaction.committed_hash();
	let mut transactions = vec![];
	let serialized_aptos_transaction = bcs::to_bytes(&signed_transaction)?;
	let movement_transaction = movement_client::movement_types::transaction::Transaction::new(
		serialized_aptos_transaction,
		0,
		signed_transaction.sequence_number(),
	);
	let serialized_transaction = serde_json::to_vec(&movement_transaction)?;
	transactions.push(BlobWrite { data: serialized_transaction });
	let batch_write = BatchWriteRequest { blobs: transactions };

	// write the batch to the DA
	let batch_write_reponse = da_client.batch_write(batch_write).await?;

	// assert that there are no intents to write the blob
	assert_eq!(batch_write_reponse.blobs.len(), 0);

	// wait for the transaction hash on the full node
	match rest_client.wait_for_transaction_by_hash(txn_hash, 20, None, None).await {
		Ok(_) => {
			println!("Transaction was successfully executed");
			anyhow::bail!("Transaction was successfully executed");
		}
		Err(e) => {
			println!("Transaction failed to execute: {:?}", e);
		}
	}

	Ok(())
}
