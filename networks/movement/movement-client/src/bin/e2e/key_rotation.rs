use anyhow::Context;
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::SigningKey;
use aptos_sdk::crypto::Uniform;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::{
	crypto::test_utils::KeyPair,
	rest_client::{Client, FaucetClient},
	types::account_address::AccountAddress,
	types::transaction::{Script, TransactionArgument, TransactionPayload},
};
use aptos_types::account_config::RotationProofChallenge;
use aptos_types::chain_id::ChainId;
use movement_client::{coin_client::CoinClient, crypto::ed25519::PublicKey, types::LocalAccount};
use once_cell::sync::Lazy;
use std::{fs, str::FromStr};
use url::Url;

/// limit of gas unit
const GAS_UNIT_LIMIT: u64 = 100000;
/// minimum price of gas unit of aptos chains
pub const GAS_UNIT_PRICE: u64 = 100;

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
	let new_keypair: KeyPair<Ed25519PrivateKey, PublicKey> =
		KeyPair::generate(&mut rand::rngs::OsRng);
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

	// Sign the rotation message directly using the private key
	let signature_by_curr_privkey = sender.private_key().sign_arbitrary_message(&rotation_message);
	let signature_by_new_privkey =
		new_keypair.private_key.sign_arbitrary_message(&rotation_message);

	// Read the compiled Move script
	let script_code = fs::read(
		"networks/movement/movement-client/src/move-modules/build/Rotate/bytecode_scripts/main.mv",
	)
	.context("Failed to read script")?;

	let script_payload = TransactionPayload::Script(Script::new(
		script_code,
		vec![], // No type arguments
		vec![
			TransactionArgument::U8(0), // Scheme for the current key (Ed25519)
			TransactionArgument::U8(0), // Scheme for the new key (Ed25519)
			TransactionArgument::U8Vector(signature_by_curr_privkey.to_bytes().to_vec()), // Signature from current key
			TransactionArgument::U8Vector(sender.public_key().to_bytes().to_vec()), // Current public key bytes
			TransactionArgument::U8Vector(new_public_key.to_bytes().to_vec()),      // New public key bytes
			TransactionArgument::U8Vector(signature_by_new_privkey.to_bytes().to_vec()), // Signature from new key
			TransactionArgument::U8Vector(vec![]), // Placeholder for `cap_update_table` (fill if applicable)
			TransactionArgument::U8(0),            // Account key scheme (Ed25519)
			TransactionArgument::U8Vector(sender.public_key().to_bytes().to_vec()), // Account public key bytes
			TransactionArgument::Address(delegate.address()), // Recipient's address for capability offer
		],
	));

	// Create and submit the transaction

	let state = rest_client
		.get_ledger_information()
		.await
		.context("Failed in getting chain id")?
		.into_inner();

	let transaction_factory = TransactionFactory::new(ChainId::new(state.chain_id))
		.with_gas_unit_price(100)
		.with_max_gas_amount(GAS_UNIT_LIMIT);

	let signed_tx = core_resources_account
		.sign_with_transaction_builder(transaction_factory.payload(script_payload));

	let response = rest_client
		.submit_and_wait(&signed_tx)
		.await
		.map_err(|e| anyhow::anyhow!(e.to_string()))?
		.into_inner();

	println!("Transaction submitted: {:?}", response);

	Ok(())
}
