use anyhow::Context;
use aptos_sdk::crypto::{SigningKey, Uniform, ValidCryptoMaterialStringExt};
use aptos_sdk::{
	crypto::test_utils::KeyPair,
	rest_client::{Client, FaucetClient},
	types::{
		account_address::AccountAddress,
		transaction::{Script, TransactionArgument, TransactionPayload},
	},
};
use aptos_types::account_config::RotationProofChallenge;
use movement_client::{coin_client::CoinClient, crypto::ed25519::PublicKey, types::LocalAccount};
use once_cell::sync::Lazy;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, str::FromStr};
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

	let associate_address = AccountAddress::from_str(
		"0x000000000000000000000000000000000000000000000000000000000a550c18",
	)?;

	let chain_id = rest_client
		.get_index()
		.await
		.context("failed to get chain ID")?
		.inner()
		.chain_id;

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
	println!(
		"resource account keypairs: {:?}, {:?}",
		core_resources_account.private_key(),
		core_resources_account.public_key()
	);
	println!("Core Resources Account address: {}", core_resources_account.address());

	tracing::info!("Created core resources account");
	// core_resources_account is already funded with u64 max value
	// Create dead account

	// Get chain ID
	let chain_id = rest_client
		.get_index()
		.await
		.context("Failed to get chain ID")?
		.inner()
		.chain_id;

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
	println!("Core Resources Account Address: {}", core_resources_account.address());
	tracing::info!("Core resources account loaded");

	// Generate sender and delegate accounts
	let mut sender = LocalAccount::generate(&mut rand::rngs::OsRng);
	let delegate = LocalAccount::generate(&mut rand::rngs::OsRng);
	tracing::info!("Generated sender and delegate accounts");

	// Fund the sender account
	faucet_client
		.fund(sender.address(), 1_000_000)
		.await
		.context("Failed to fund sender account")?;

	// Generate new key pair for rotation using KeyPair
	let new_keypair: KeyPair<_, PublicKey> = KeyPair::generate(&mut rand::rngs::OsRng);
	let new_public_key: PublicKey = new_keypair.public_key.clone();

	// Create the rotation proof challenge
	let rotation_proof = RotationProofChallenge {
		module_name: String::from("account"),
		struct_name: String::from("RotationProofChallenge"),
		account_address: sender.address(),
		sequence_number: sender.sequence_number(),
		originator: sender.address(),
		current_auth_key: AccountAddress::from_str(
			core_resources_account.private_key().to_encoded_string().unwrap().as_str(),
		)?,
		new_public_key: Vec::from(new_public_key.to_bytes()),
	};

	let rotation_message = bcs::to_bytes(&rotation_proof).unwrap();

	// Sign the rotation proof challenge
	let signature_by_new_key = new_keypair.private_key.sign(&rotation_message);

	// Read the compiled Move script
	let script_code = fs::read("path/to/compiled/script.mv").context("Failed to read script")?;
	let script_payload = TransactionPayload::Script(Script::new(
		script_code,
		vec![],
		vec![
			TransactionArgument::U8(0), // Scheme for the current key (Ed25519)
			TransactionArgument::U8(0), // Scheme for the new key (Ed25519)
			TransactionArgument::Bytes(new_public_key.to_bytes().to_vec()),
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
		chain_id,
	);

	tracing::info!("Submitting transaction for key rotation");
	rest_client
		.submit_and_wait(&txn)
		.await
		.context("Failed to submit key rotation transaction")?;

	tracing::info!("Key rotation transaction completed successfully");

	Ok(())
}
