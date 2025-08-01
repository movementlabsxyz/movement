use anyhow::Context;
use aptos_sdk::{
	coin_client::CoinClient,
	crypto::{SigningKey, ValidCryptoMaterialStringExt},
	move_types::{
		identifier::Identifier,
		language_storage::{ModuleId, TypeTag},
	},
	rest_client::{Client, FaucetClient, Transaction},
	transaction_builder::TransactionFactory,
	types::{account_address::AccountAddress, transaction::TransactionPayload},
};
use aptos_types::{
	account_config::{RotationProofChallenge, CORE_CODE_ADDRESS},
	chain_id::ChainId,
	transaction::EntryFunction,
};
use movement_client::types::LocalAccount;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
//use tokio::process::Command;
use movement_client::crypto::ed25519::Ed25519PrivateKey;
use tracing::info;
use tracing_subscriber::EnvFilter;
use url::Url;

/// limit of gas unit
const GAS_UNIT_LIMIT: u64 = 100000;
/// minimum price of gas unit of aptos chains
pub const GAS_UNIT_PRICE: u64 = 100;

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap()
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let node_connection_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port =
		SUZUKA_CONFIG.execution_config.maptos_config.client.maptos_rest_connection_port;
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
		.maptos_faucet_rest_connection_port;

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
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Initialize clients
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);

	// Load core resource account
	let raw_private_key = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.chain
		.maptos_private_key_signer_identifier
		.try_raw_private_key()?;
	let private_key = Ed25519PrivateKey::try_from(raw_private_key.as_slice())?;
	let mut core_resources_account =
		LocalAccount::from_private_key(private_key.to_encoded_string()?.as_str(), 0)?;
	info!(
		"Core Resources Account keypairs: {:?}, {:?}",
		core_resources_account.private_key(),
		core_resources_account.public_key()
	);
	info!("Core Resources Account address: {}", core_resources_account.address());

	// Fund the account
	faucet_client.fund(core_resources_account.address(), 100_000_000_000).await?;

	let state = rest_client
		.get_ledger_information()
		.await
		.context("Failed in getting chain id")?
		.into_inner();

	// Generate recipient account
	let recipient = LocalAccount::generate(&mut rand::rngs::OsRng);

	faucet_client.fund(recipient.address(), 100_000_000_000).await?;

	let recipient_bal = coin_client
		.get_account_balance(&recipient.address())
		.await
		.context("Failed to get recipient's account balance")?;

	let core_resource_bal = coin_client
		.get_account_balance(&core_resources_account.address())
		.await
		.context("Failed to get core resources account balance")?;

	info!("Recipient's balance: {:?}", recipient_bal);
	info!("Core Resources Account balance: {:?}", core_resource_bal);

	// --- Offer Rotation Capability ---
	let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
		account_address: CORE_CODE_ADDRESS,
		module_name: String::from("account"),
		struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
		chain_id: state.chain_id,
		sequence_number: core_resources_account.increment_sequence_number(),
		source_address: core_resources_account.address(),
		recipient_address: recipient.address(),
	};

	let rotation_capability_proof_msg = bcs::to_bytes(&rotation_capability_proof)
		.context("Failed to serialize rotation capability proof challenge")?;
	let rotation_proof_signed = core_resources_account
		.private_key()
		.sign_arbitrary_message(&rotation_capability_proof_msg);

	let is_valid = verify_signature(
		&core_resources_account.public_key().to_bytes(),
		&rotation_capability_proof_msg,
		&rotation_proof_signed.to_bytes(),
	)?;

	assert!(is_valid, "Signature verification failed!");
	info!("Signature successfully verified!");

	let offer_payload = make_entry_function_payload(
		CORE_CODE_ADDRESS,
		"account",
		"offer_rotation_capability",
		vec![],
		vec![
			bcs::to_bytes(&rotation_proof_signed.to_bytes().to_vec())
				.context("Failed to serialize rotation capability signature")?,
			bcs::to_bytes(&0u8).context("Failed to serialize account scheme")?,
			bcs::to_bytes(&core_resources_account.public_key().to_bytes().to_vec())
				.context("Failed to serialize public key bytes")?,
			bcs::to_bytes(&recipient.address()).context("Failed to serialize recipient address")?,
		],
	)?;

	core_resources_account.decrement_sequence_number();

	let offer_response =
		send_aptos_transaction(&rest_client, &mut core_resources_account, offer_payload).await?;
	info!("Offer transaction response: {:?}", offer_response);

	// --- Rotate Authentication Key ---
	let rotation_proof = RotationProofChallenge {
		account_address: CORE_CODE_ADDRESS,
		module_name: String::from("account"),
		struct_name: String::from("RotationProofChallenge"),
		sequence_number: core_resources_account.increment_sequence_number(),
		originator: core_resources_account.address(),
		current_auth_key: AccountAddress::from_bytes(core_resources_account.authentication_key())?,
		new_public_key: recipient.public_key().to_bytes().to_vec(),
	};

	let rotation_message =
		bcs::to_bytes(&rotation_proof).context("Failed to serialize rotation proof challenge")?;

	let signature_by_curr_privkey =
		core_resources_account.private_key().sign_arbitrary_message(&rotation_message);
	let signature_by_new_privkey =
		recipient.private_key().sign_arbitrary_message(&rotation_message);

	let rotate_payload = make_entry_function_payload(
		AccountAddress::from_hex_literal("0x1").context("Invalid hex literal for account")?,
		"account",
		"rotate_authentication_key",
		vec![],
		vec![
			bcs::to_bytes(&0u8).context("Failed to serialize from_scheme")?,
			bcs::to_bytes(&core_resources_account.public_key().to_bytes().to_vec())
				.context("Failed to serialize from_public_key_bytes")?,
			bcs::to_bytes(&0u8).context("Failed to serialize to_scheme")?,
			bcs::to_bytes(&recipient.public_key().to_bytes().to_vec())
				.context("Failed to serialize to_public_key_bytes")?,
			bcs::to_bytes(&signature_by_curr_privkey.to_bytes().to_vec())
				.context("Failed to serialize cap_rotate_key")?,
			bcs::to_bytes(&signature_by_new_privkey.to_bytes().to_vec())
				.context("Failed to serialize cap_update_table")?,
		],
	)?;

	core_resources_account.decrement_sequence_number();

	let rotate_response =
		send_aptos_transaction(&rest_client, &mut core_resources_account, rotate_payload).await?;
	info!("Rotate transaction response: {:?}", rotate_response);

	Ok(())
}

fn make_entry_function_payload(
	package_address: AccountAddress,
	module_name: &'static str,
	function_name: &'static str,
	ty_args: Vec<TypeTag>,
	args: Vec<Vec<u8>>,
) -> Result<TransactionPayload, anyhow::Error> {
	tracing::info!("Creating entry function payload for package address: {:?}", package_address);

	let module_id = ModuleId::new(
		package_address,
		Identifier::new(module_name).context("Invalid module name")?,
	);

	let function_id = Identifier::new(function_name).context("Invalid function name")?;

	Ok(TransactionPayload::EntryFunction(EntryFunction::new(module_id, function_id, ty_args, args)))
}

fn verify_signature(
	public_key_bytes: &[u8; 32],
	message: &[u8],
	signature_bytes: &[u8; 64],
) -> Result<bool, anyhow::Error> {
	use ed25519_dalek::{Signature, Verifier, VerifyingKey};

	let verifying_key =
		VerifyingKey::from_bytes(public_key_bytes).context("Failed to parse public key bytes")?;

	let signature = Signature::from_bytes(signature_bytes);

	Ok(verifying_key.verify(message, &signature).is_ok())
}

async fn send_aptos_transaction(
	client: &Client,
	signer: &mut LocalAccount,
	payload: TransactionPayload,
) -> anyhow::Result<Transaction> {
	let state = client
		.get_ledger_information()
		.await
		.context("Failed to retrieve ledger information")?
		.into_inner();

	let transaction_factory = TransactionFactory::new(ChainId::new(state.chain_id))
		.with_gas_unit_price(100)
		.with_max_gas_amount(GAS_UNIT_LIMIT);

	let signed_tx = signer.sign_with_transaction_builder(transaction_factory.payload(payload));

	let response = client
		.submit_and_wait(&signed_tx)
		.await
		.context("Failed to submit and wait for transaction")?
		.into_inner();

	Ok(response)
}
