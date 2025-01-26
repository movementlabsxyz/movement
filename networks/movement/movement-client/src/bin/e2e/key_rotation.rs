#![allow(unused_imports)]
use anyhow::Context;
use aptos_sdk::coin_client::CoinClient;
//use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::SigningKey;
use aptos_sdk::crypto::Uniform;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::move_types::language_storage::TypeTag;
use aptos_sdk::rest_client::FaucetClient;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::{
	crypto::test_utils::KeyPair, rest_client::Client, types::account_address::AccountAddress,
	types::transaction::TransactionPayload,
};
use aptos_types::account_config::RotationProofChallenge;
use aptos_types::account_config::CORE_CODE_ADDRESS;
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::EntryFunction;
use movement_client::{crypto::ed25519::PublicKey, types::LocalAccount};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use url::Url;

/// limit of gas unit
//const GAS_UNIT_LIMIT: u64 = 100000;
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

	Url::from_str(faucet_listen_url.as_str()).unwrap()
});

#[derive(Serialize, Deserialize)]
struct RotationCapabilityOfferProofChallengeV2 {
	account_address: AccountAddress,
	module_name: String,
	struct_name: String,
	chain_id: u8,
	sequence_number: u64,
	source_address: AccountAddress,
	recipient_address: AccountAddress,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Initialize clients
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	// Load core resource account
	let core_resources_account = LocalAccount::from_private_key(
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
		"Core Resources Account keypairs: {:?}, {:?}",
		core_resources_account.private_key(),
		core_resources_account.public_key()
	);
	println!("Core Resources Account address: {}", core_resources_account.address());

	// Fund the account
	faucet_client.fund(core_resources_account.address(), 100_000_000_000).await?;

	let state = rest_client
		.get_ledger_information()
		.await
		.context("Failed in getting chain id")?
		.into_inner();

	//Generate recepient account
	let recipient = LocalAccount::generate(&mut rand::rngs::OsRng);

	let recipient_seq_num = 0_u64;

	faucet_client.fund(recipient.address(), 100_000_000_000).await?;

	let recipient_bal = coin_client
		.get_account_balance(&recipient.address())
		.await
		.context("Failed to get recipient's account balance")?;

	let core_resource_bal = coin_client
		.get_account_balance(&core_resources_account.address())
		.await
		.context("Failed to get core resources account balance")?;

	println!("Recipient's balance: {:?}", recipient_bal);
	println!("Core Resources Account balance: {:?}", core_resource_bal);

	// --- Offer Rotation Capability ---
	let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
		account_address: CORE_CODE_ADDRESS,
		module_name: String::from("account"),
		struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
		chain_id: state.chain_id,
		sequence_number: core_resources_account.sequence_number(),
		source_address: core_resources_account.address(),
		recipient_address: recipient.address(),
	};

	// Serialize the rotation capability proof challenge
	let rotation_capability_proof_msg = bcs::to_bytes(&rotation_capability_proof).unwrap();
	let rotation_proof_signed = core_resources_account
		.private_key()
		.sign_arbitrary_message(&rotation_capability_proof_msg);

	let offer_payload = make_entry_function_payload(
		CORE_CODE_ADDRESS,           // Package address
		"account",                   // Module name
		"offer_rotation_capability", // Function name
		vec![],                      // Type arguments
		vec![
			bcs::to_bytes(&rotation_proof_signed.to_bytes().to_vec()).unwrap(), // rotation_capability_sig_bytes
			bcs::to_bytes(&0u8).unwrap(),                                       // account_scheme (Ed25519)
			bcs::to_bytes(&core_resources_account.public_key().to_bytes()).unwrap(), // account_public_key_bytes
			bcs::to_bytes(&recipient.address()).unwrap(),                       // recipient_address
		],
	);

	println!("Offer Payload: {:?}", offer_payload);

	// Submit the offer transaction
	let offer_signed_tx = core_resources_account.sign_with_transaction_builder(
		TransactionFactory::new(ChainId::new(state.chain_id)).payload(offer_payload),
	);

	println!("Offer signed tx: {:?}", offer_signed_tx);

	let offer_response = rest_client
		.submit_and_wait(&offer_signed_tx)
		.await
		.map_err(|e| anyhow::anyhow!(e.to_string()))?
		.into_inner();

	println!("Offer transaction response: {:?}", offer_response);

	// --- Rotate Authentication Key ---
	let rotation_proof = RotationProofChallenge {
		module_name: String::from("account"),
		struct_name: String::from("RotationProofChallenge"),
		account_address: core_resources_account.address(),
		originator: core_resources_account.address(),
		current_auth_key: AccountAddress::from_str(
			core_resources_account
				.authentication_key()
				.to_encoded_string()
				.unwrap()
				.as_str(),
		)?,
		new_public_key: Vec::from(recipient.public_key().to_bytes()),
		sequence_number: core_resources_account.sequence_number(),
	};

	let rotation_message = bcs::to_bytes(&rotation_proof).unwrap();
	let signature_by_curr_privkey =
		core_resources_account.private_key().sign_arbitrary_message(&rotation_message);
	let signature_by_new_privkey =
		recipient.private_key().sign_arbitrary_message(&rotation_message);

	let rotate_payload = make_entry_function_payload(
		AccountAddress::from_hex_literal("0x1").unwrap(), // Package address
		"account",                                        // Module name
		"rotate_authentication_key",                      // Function name
		vec![],                                           // Type arguments
		vec![
			bcs::to_bytes(&0u8).unwrap(), // from_scheme (Ed25519)
			bcs::to_bytes(&core_resources_account.public_key().to_bytes().to_vec()).unwrap(), // from_public_key_bytes
			bcs::to_bytes(&0u8).unwrap(), // to_scheme (Ed25519)
			bcs::to_bytes(&recipient.public_key().to_bytes().to_vec()).unwrap(), // to_public_key_bytes
			bcs::to_bytes(&signature_by_curr_privkey.to_bytes().to_vec()).unwrap(), // cap_rotate_key
			bcs::to_bytes(&signature_by_new_privkey.to_bytes().to_vec()).unwrap(), // cap_update_table (signature by new private key)
		],
	);

	println!("Rotate Payload: {:?}", rotate_payload);

	// Submit the rotation transaction
	let rotate_signed_tx = core_resources_account.sign_with_transaction_builder(
		TransactionFactory::new(ChainId::new(state.chain_id)).payload(rotate_payload),
	);

	let rotate_response = rest_client
		.submit_and_wait(&rotate_signed_tx)
		.await
		.map_err(|e| anyhow::anyhow!(e.to_string()))?
		.into_inner();

	println!("Rotation transaction response: {:?}", rotate_response);

	Ok(())
}

fn make_entry_function_payload(
	package_address: AccountAddress,
	module_name: &'static str,
	function_name: &'static str,
	ty_args: Vec<TypeTag>,
	args: Vec<Vec<u8>>,
) -> TransactionPayload {
	println!("package_address: {:?}", package_address);
	TransactionPayload::EntryFunction(EntryFunction::new(
		ModuleId::new(package_address, Identifier::new(module_name).unwrap()),
		Identifier::new(function_name).unwrap(),
		ty_args,
		args,
	))
}
