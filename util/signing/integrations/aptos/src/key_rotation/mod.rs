pub mod signer;
use anyhow::Context;
use aptos_crypto::ValidCryptoMaterial;
use aptos_sdk::rest_client::Client;
use aptos_sdk::rest_client::Transaction;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::{
	move_types::{
		identifier::Identifier,
		language_storage::{ModuleId, TypeTag},
	},
	types::transaction::TransactionPayload,
};
use aptos_types::transaction::RawTransaction;
use aptos_types::{
	account_config::{RotationProofChallenge, CORE_CODE_ADDRESS},
	chain_id::ChainId,
	transaction::EntryFunction,
};
use serde::{Deserialize, Serialize};
use signer::{KeyRotationSigner, KeyRotationSignerError};
use std::error;
use tracing::info;

#[derive(Debug, thiserror::Error)]
pub enum KeyRotationError {
	#[error("failed to sign key rotation: {0}")]
	Signing(#[from] KeyRotationSignerError),
	#[error("key rotation transaction failed: {0}")]
	TransactionFailed(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("failed to rotate keys: {0}")]
	RotationFailed(#[source] Box<dyn error::Error + Send + Sync>),
}

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
pub struct KeyRotator;

impl KeyRotator {
	pub fn new() -> Self {
		Self
	}

	/// Rotates keys without a 2PC protocol. Jokingly called "1PC" protocol.
	pub async fn rotate_key_1pc(
		&self,
		client: &Client,
		old_signer: &impl KeyRotationSigner,
		new_signer: &impl KeyRotationSigner,
	) -> Result<(), KeyRotationError> {
		// get state information from the chain
		let state = client
			.get_ledger_information()
			.await
			.context("Failed in getting chain id")
			.map_err(|e| KeyRotationError::RotationFailed(e.into()))?
			.into_inner();

		// get the information from the old signer
		let old_key_rotation_sequence_number =
			old_signer.key_rotation_account_sequence_number(client).await?;
		let old_address = old_signer.key_rotation_account_address(client).await?;
		let old_public_key = old_signer.public_key().await?;
		let authentication_key = old_signer.key_rotation_account_authentication_key().await?;

		// get the information from the new signer
		let new_key_rotation_address = new_signer.key_rotation_account_address(client).await?;
		let new_public_key = new_signer.public_key().await?;

		// --- Offer Rotation Capability ---
		let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
			account_address: CORE_CODE_ADDRESS,
			module_name: String::from("account"),
			struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
			chain_id: state.chain_id,
			sequence_number: old_key_rotation_sequence_number,
			source_address: old_address,
			recipient_address: new_key_rotation_address,
		};

		let rotation_capability_proof_msg =
			bcs::to_bytes(&rotation_capability_proof).map_err(|e| {
				KeyRotationError::RotationFailed(
					format!("failed to serialize rotation capability proof challenge: {:?}", e)
						.into(),
				)
			})?;
		let rotation_proof_signed = old_signer.sign_message(&rotation_capability_proof_msg).await?;

		let is_valid = verify_signature(
			&old_public_key.to_bytes(),
			&rotation_capability_proof_msg,
			&rotation_proof_signed.to_bytes(),
		)
		.map_err(|e| KeyRotationError::RotationFailed(e.into()))?;

		assert!(is_valid, "Signature verification failed!");
		info!("Signature successfully verified!");

		let offer_payload = make_entry_function_payload(
			CORE_CODE_ADDRESS,
			"account",
			"offer_rotation_capability",
			vec![],
			vec![
				bcs::to_bytes(&rotation_proof_signed.to_bytes().to_vec())
					.context("failed to serialize rotation capability signature")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
				bcs::to_bytes(&0u8)
					.context("failed to serialize account scheme")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
				bcs::to_bytes(&old_public_key.to_bytes().to_vec())
					.context("Failed to serialize public key bytes")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
				bcs::to_bytes(&new_key_rotation_address)
					.context("Failed to serialize recipient address")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
			],
		)
		.map_err(|e| KeyRotationError::RotationFailed(e.into()))?;

		let offer_response = send_aptos_transaction_default(&client, old_signer, offer_payload)
			.await
			.map_err(|e| KeyRotationError::TransactionFailed(e.into()))?;
		info!("Offer transaction response: {:?}", offer_response);

		// --- Rotate Authentication Key ---
		let rotation_proof = RotationProofChallenge {
			account_address: CORE_CODE_ADDRESS,
			module_name: String::from("account"),
			struct_name: String::from("RotationProofChallenge"),
			sequence_number: old_key_rotation_sequence_number + 1,
			originator: old_address,
			current_auth_key: AccountAddress::from_bytes(
				authentication_key.to_bytes().to_vec().as_slice(),
			)
			.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
			new_public_key: new_public_key.to_bytes().to_vec(),
		};

		let rotation_message = bcs::to_bytes(&rotation_proof).map_err(|e| {
			KeyRotationError::RotationFailed(
				format!("failed to serialize rotation proof challenge: {:?}", e).into(),
			)
		})?;

		let signature_by_curr_privkey =
			old_signer.sign_message(&rotation_message).await.map_err(|e| {
				KeyRotationError::RotationFailed(
					format!("failed to sign rotation proof challenge: {:?}", e).into(),
				)
			})?;
		let signature_by_new_privkey =
			new_signer.sign_message(&rotation_message).await.map_err(|e| {
				KeyRotationError::RotationFailed(
					format!("failed to sign rotation proof challenge: {:?}", e).into(),
				)
			})?;

		let rotate_payload = make_entry_function_payload(
			AccountAddress::from_hex_literal("0x1")
				.context("Invalid hex literal for account")
				.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
			"account",
			"rotate_authentication_key",
			vec![],
			vec![
				bcs::to_bytes(&0u8)
					.context("failed to serialize from_scheme")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
				bcs::to_bytes(&old_public_key.to_bytes().to_vec())
					.context("failed to serialize from_public_key_bytes")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
				bcs::to_bytes(&0u8)
					.context("failed to serialize to_scheme")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
				bcs::to_bytes(&new_public_key.to_bytes().to_vec())
					.context("failed to serialize to_public_key_bytes")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
				bcs::to_bytes(&signature_by_curr_privkey.to_bytes().to_vec())
					.context("failed to serialize cap_rotate_key")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
				bcs::to_bytes(&signature_by_new_privkey.to_bytes().to_vec())
					.context("failed to serialize cap_update_table")
					.map_err(|e| KeyRotationError::RotationFailed(e.into()))?,
			],
		)
		.map_err(|e| KeyRotationError::RotationFailed(e.into()))?;

		let rotate_response = send_aptos_transaction_default(&client, old_signer, rotate_payload)
			.await
			.map_err(|e| KeyRotationError::TransactionFailed(e.into()))?;
		info!("Rotate transaction response: {:?}", rotate_response);

		// Rotate the signing keys
		Ok(())
	}
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
	signer: &impl KeyRotationSigner,
	account_address: AccountAddress,
	sequence_number: u64,
	max_gas_amount: u64,
	gas_unit_price: u64,
	expiration_timestamp_sec_offset: u64,
	chain_id: ChainId,
	payload: TransactionPayload,
) -> anyhow::Result<Transaction> {
	// get the current time in seconds and add the offset
	let now_u64 = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map_err(|e| anyhow::anyhow!("failed to get current time {}", e))?
		.as_secs();
	let expiration_timestamp_secs = now_u64 + expiration_timestamp_sec_offset;

	// form the raw transaction
	let raw_transaction = RawTransaction::new(
		account_address,
		sequence_number,
		payload,
		max_gas_amount,
		gas_unit_price,
		expiration_timestamp_secs,
		chain_id,
	);

	// sign the transaction
	let signed_tx = signer.sign_key_rotation(raw_transaction).await?;

	// submit and wait for the transaction
	let response = client
		.submit_and_wait(&signed_tx)
		.await
		.context("Failed to submit and wait for transaction")?
		.into_inner();

	Ok(response)
}

async fn send_aptos_transaction_default(
	client: &Client,
	signer: &impl KeyRotationSigner,
	payload: TransactionPayload,
) -> anyhow::Result<Transaction> {
	// get the chain id
	let ledger_information = client
		.get_ledger_information()
		.await
		.map_err(|e| anyhow::anyhow!("failed to get ledger information: {}", e))?;
	let chain_id = ChainId::new(ledger_information.into_inner().chain_id);

	// get the account address and sequence number
	let account_address = signer.key_rotation_account_address(client).await?;
	let sequence_number = signer.key_rotation_account_sequence_number(client).await?;

	// use the default values for gas and expiration
	let max_gas_amount = 100_000;
	let gas_unit_price = 10;
	let expiration_timestamp_secs_offset = 60;

	// send the transaction
	let response = send_aptos_transaction(
		client,
		signer,
		account_address,
		sequence_number,
		max_gas_amount,
		gas_unit_price,
		expiration_timestamp_secs_offset,
		chain_id,
		payload,
	)
	.await?;

	Ok(response)
}
