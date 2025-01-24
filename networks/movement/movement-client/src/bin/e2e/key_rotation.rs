use anyhow::Context;
use aptos_sdk::{
	rest_client::{Client, FaucetClient, Response},
	types::{
		account_address::AccountAddress,
		transaction::{Script, TransactionArgument, TransactionPayload},
	},
};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
use movement_client::{
	coin_client::CoinClient,
	types::{LocalAccount, RotationProofChallenge},
};
use once_cell::sync::Lazy;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, str::FromStr};
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
		.maptos_faucet_rest_connection_port
		.clone();
	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);
	Url::from_str(&faucet_listen_url).unwrap()
});

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Initialize clients
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	// Generate accounts
	let mut sender = LocalAccount::generate(&mut rand::rngs::OsRng);
	let delegate = LocalAccount::generate(&mut rand::rngs::OsRng);

	tracing::info!("Generated sender and delegate accounts");

	// Fund the sender account
	faucet_client
		.fund(sender.address(), 1_000_000)
		.await
		.context("Failed to fund sender account")?;

	// Generate new key pair for rotation
	let new_keypair = Keypair::generate(&mut rand::rngs::OsRng);
	let new_public_key: PublicKey = new_keypair.public;

	// Create the rotation proof challenge
	let rotation_proof = RotationProofChallenge {
		account_address: sender.address(),
		sequence_number: sender.sequence_number(),
		originator: sender.address(),
		current_auth_key: sender.auth_key().to_vec(),
		new_public_key: new_public_key.as_bytes().to_vec(),
	};

	let rotation_message = bcs::to_bytes(&rotation_proof).unwrap();

	// Sign the rotation proof challenge
	let signature_by_new_key = new_keypair.sign(&rotation_message);

	// Read compiled Move script
	let script_code = fs::read("path/to/compiled/script.mv").context("Failed to read script")?;
	let script_payload = TransactionPayload::Script(Script::new(
		script_code,
		vec![],
		vec![
			TransactionArgument::U8(0), // Scheme for the current key (Ed25519)
			TransactionArgument::U8(0), // Scheme for the new key (Ed25519)
			TransactionArgument::Bytes(new_public_key.as_bytes().to_vec()),
			TransactionArgument::Bytes(signature_by_new_key.to_bytes().to_vec()),
		],
	));

	// Create and submit the transaction
	let expiration_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60; // 60 seconds from now

	let txn = transaction_test_helpers::get_test_signed_transaction_with_chain_id(
		sender.address(),
		sender.sequence_number(),
		sender.private_key(),
		sender.public_key(),
		Some(script_payload),
		expiration_time,
		100, // Max gas
		None,
		ChainId::new(1), // Set chain ID
	);

	tracing::info!("Submitting transaction for key rotation");
	rest_client
		.submit_and_wait(&txn)
		.await
		.context("Failed to submit key rotation transaction")?;

	tracing::info!("Key rotation transaction completed successfully");

	Ok(())
}
