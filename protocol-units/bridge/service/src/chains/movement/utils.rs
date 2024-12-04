use anyhow::{Context, Result};
use aptos_sdk::{
	crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature},
	move_types::{
		account_address::AccountAddressParseError,
		ident_str,
		language_storage::{ModuleId, TypeTag},
	},
	rest_client::{
		aptos_api_types::{
			EntryFunctionId, MoveType, Transaction as AptosTransaction, TransactionInfo,
			ViewRequest,
		},
		Client as RestClient, FaucetClient, Transaction,
	},
	transaction_builder::TransactionFactory,
	types::{
		account_address::AccountAddress,
		chain_id::ChainId,
		transaction::{EntryFunction, SignedTransaction, TransactionPayload},
		AccountKey, LocalAccount,
	},
};
use bridge_util::chains::AddressVecCodec;
use bridge_util::{
	chains::bridge_contracts::BridgeContractError,
	types::{AddressError, BridgeAddress},
};
use derive_new::new;
use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use thiserror::Error;
use tiny_keccak::{Hasher, Keccak};
use tracing::log::{error, info};
use url::Url;

use super::client_framework::MovementClientFramework;
pub type TestRng = StdRng;

const MOVEMENT_RPC_URL: &str = "https://testnet.bardock.movementnetwork.xyz";
const MOVEMENT_FAUCET_URL: &str = "https://faucet.testnet.bardock.movementnetwork.xyz";

pub trait RngSeededClone: Rng + SeedableRng {
	fn seeded_clone(&mut self) -> Self;
}

impl RngSeededClone for StdRng {
	fn seeded_clone(&mut self) -> Self {
		self.clone()
	}
}

impl RngSeededClone for ChaChaRng {
	fn seeded_clone(&mut self) -> Self {
		let mut seed = [0u8; 32];
		self.fill_bytes(&mut seed);
		ChaChaRng::from_seed(seed)
	}
}

#[derive(Debug, Error)]
pub enum MovementAddressError {
	#[error("Invalid hex string")]
	InvalidHexString,
	#[error("Invalid byte length for AccountAddress")]
	InvalidByteLength,
	#[error("Invalid AccountAddress")]
	AccountParseError(#[from] AccountAddressParseError),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct MovementAddress(pub AccountAddress);

impl From<MovementAddress> for Vec<u8> {
	fn from(address: MovementAddress) -> Vec<u8> {
		address.0.into()
	}
}

impl From<BridgeAddress<MovementAddress>> for MovementAddress {
	fn from(address: BridgeAddress<MovementAddress>) -> Self {
		address.0
	}
}

impl From<&MovementAddress> for Vec<u8> {
	fn from(address: &MovementAddress) -> Vec<u8> {
		address.0.to_vec()
	}
}

impl FromStr for MovementAddress {
	type Err = MovementAddressError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		AccountAddress::from_str(s).map(MovementAddress).map_err(From::from)
	}
}

impl std::fmt::Display for MovementAddress {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0.to_standard_string())
	}
}

impl TryFrom<Vec<u8>> for MovementAddress {
	type Error = AddressError;

	fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
		if vec.len() != AccountAddress::LENGTH {
			return Err(AddressError::InvalidByteLength(vec.len()));
		}
		AccountAddress::from_bytes(vec).map(MovementAddress).map_err(|_| {
			AddressError::AddressConvertionlError(
				"MovementAddress try_from AccountAddress conversion error".to_string(),
			)
		})
	}
}

impl TryFrom<&str> for MovementAddress {
	type Error = AddressError;

	fn try_from(s: &str) -> Result<Self, Self::Error> {
		let s = s.trim_start_matches("0x");
		let bytes = hex::decode(s).map_err(|_| AddressError::InvalidHexString)?;
		bytes.try_into()
	}
}

impl AddressVecCodec for MovementAddress {
	fn try_decode_recipient(value: Vec<u8>) -> Result<Self, AddressError> {
		// Get binary address from movement chain.
		MovementAddress::try_from(value)
	}
	fn encode_initiator(self) -> Vec<u8> {
		let bytes: Vec<u8> = self.into();
		// convert initiator address to hex because Transfer id verification
		// use hex encoded address for initiator on Mvt.
		// Encode initiator so that counterpart chain use the same encoding.
		hex::encode(bytes).into_bytes()
	}
}

/// limit of gas unit
const GAS_UNIT_LIMIT: u64 = 100000;
/// minimum price of gas unit of aptos chains
pub const GAS_UNIT_PRICE: u64 = 100;

/// Wrapper struct that adds indexing information to a type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, new)]
pub struct Indexed<T> {
	inner: T,
	#[new(default)]
	/// Optional sequence data that is useful during indexing
	pub sequence: Option<u32>,
}

/// Send Aptos Transaction
pub async fn send_and_confirm_aptos_transaction(
	rest_client: &RestClient,
	signer: &LocalAccount,
	payload: TransactionPayload,
) -> Result<AptosTransaction, String> {
	info!("Starting send_aptos_transaction");
	let state = rest_client
		.get_ledger_information()
		.await
		.map_err(|e| format!("Failed in getting chain id: {}", e))?
		.into_inner();

	let transaction_factory = TransactionFactory::new(ChainId::new(state.chain_id))
		.with_gas_unit_price(100)
		.with_max_gas_amount(GAS_UNIT_LIMIT);
	let latest_account_info = rest_client
		.get_account(signer.address())
		.await
		.map_err(|e| format!("Failed to get account information: {}", e))?;
	let account = latest_account_info.into_inner();
	let latest_sequence_number = account.sequence_number;

	let raw_tx = transaction_factory
		.payload(payload)
		.sender(signer.address())
		.sequence_number(latest_sequence_number)
		.build();

	let signed_tx = signer.sign_transaction(raw_tx);

	info!("Signed TX: {:?}", signed_tx);

	let response = rest_client.submit_and_wait(&signed_tx).await.map_err(|e| {
		let err_msg = format!("Transaction submission error: {}", e.to_string());
		error!("Full error: {}", err_msg); // Log the error in detail
		err_msg
	})?;

	let txn = response.into_inner();
	info!("Response: {:?}", txn);

	match &txn {
		Transaction::UserTransaction(user_txn) => {
			if !user_txn.info.success {
				return Err(format!("Transaction failed with status: {}", user_txn.info.vm_status));
			}
		}
		_ => {
			return Err(
				"Expected a UserTransaction, but got a different transaction type.".to_string()
			)
		}
	}

	Ok(txn)
}

pub fn extract_bridge_transfer_id(txn: Transaction) -> Option<String> {
	if let Transaction::UserTransaction(user_txn) = txn {
		for event in user_txn.events {
			// Extract the event type as a string to compare it
			let event_type = event.typ.to_string();
			if event_type.contains("BridgeTransferInitiatedEvent") {
				if let Some(Value::String(bridge_transfer_id)) =
					event.data.get("bridge_transfer_id")
				{
					return Some(bridge_transfer_id.clone());
				}
			}
		}
	}

	None
}

pub fn val_as_str(value: Option<&Value>) -> Result<&str, BridgeContractError> {
	value
		.as_ref()
		.and_then(|v| v.as_str())
		.ok_or(BridgeContractError::SerializationError)
}

pub fn val_as_u64(value: Option<&Value>) -> Result<u64, BridgeContractError> {
	value
		.as_ref()
		.and_then(|v| v.as_u64())
		.ok_or(BridgeContractError::SerializationError)
}

pub fn val_as_str_initiator(value: Option<&Value>) -> Result<&str, BridgeContractError> {
	value
		.as_ref()
		.and_then(|v| v.as_str())
		.ok_or(BridgeContractError::SerializationError)
}

pub fn val_as_u64_initiator(value: Option<&Value>) -> Result<u64, BridgeContractError> {
	value
		.as_ref()
		.and_then(|v| v.as_u64())
		.ok_or(BridgeContractError::SerializationError)
}

pub fn serialize_u64(value: &u64) -> Result<Vec<u8>, BridgeContractError> {
	bcs::to_bytes(value).map_err(|_| BridgeContractError::SerializationError)
}

pub fn serialize_vec<T: serde::Serialize + ?Sized>(
	value: &T,
) -> Result<Vec<u8>, BridgeContractError> {
	bcs::to_bytes(value).map_err(|_| BridgeContractError::SerializationError)
}

pub fn serialize_u64_initiator(value: u64) -> Result<Vec<u8>, BridgeContractError> {
	bcs::to_bytes(&value).map_err(|_| BridgeContractError::SerializationError)
}

pub fn serialize_u128_initiator(value: u128) -> Result<Vec<u8>, BridgeContractError> {
	bcs::to_bytes(&value).map_err(|_| BridgeContractError::SerializationError)
}

pub fn serialize_address_initiator(
	address: &AccountAddress,
) -> Result<Vec<u8>, BridgeContractError> {
	bcs::to_bytes(address).map_err(|_| BridgeContractError::SerializationError)
}

pub fn serialize_vec_initiator<T: serde::Serialize + ?Sized>(
	value: &T,
) -> Result<Vec<u8>, BridgeContractError> {
	bcs::to_bytes(value).map_err(|_| BridgeContractError::SerializationError)
}

pub async fn simulate_aptos_transaction(
	aptos_client: &MovementClientFramework,
	signer: &mut LocalAccount,
	payload: TransactionPayload,
) -> Result<TransactionInfo> {
	let state = aptos_client
		.rest_client
		.get_ledger_information()
		.await
		.context("Failed in getting chain id")?
		.into_inner();

	let transaction_factory = TransactionFactory::new(ChainId::new(state.chain_id))
		.with_gas_unit_price(GAS_UNIT_PRICE)
		.with_max_gas_amount(GAS_UNIT_LIMIT);

	let latest_account_info = aptos_client.rest_client.get_account(signer.address()).await?;
	let account = latest_account_info.into_inner();
	let latest_sequence_number = account.sequence_number;

	let raw_tx = transaction_factory
		.payload(payload)
		.sender(signer.address())
		.sequence_number(latest_sequence_number)
		.build();

	let signed_tx = SignedTransaction::new(
		raw_tx,
		signer.public_key().clone(),
		Ed25519Signature::try_from([0u8; 64].as_ref())?,
	);

	let response_txns = aptos_client.rest_client.simulate(&signed_tx).await?.into_inner();
	let response = response_txns[0].clone();

	Ok(response.info)
}

/// Make Aptos Transaction Payload
pub fn make_aptos_payload(
	package_address: AccountAddress,
	module_name: &'static str,
	function_name: &'static str,
	ty_args: Vec<TypeTag>,
	args: Vec<Vec<u8>>,
) -> TransactionPayload {
	TransactionPayload::EntryFunction(EntryFunction::new(
		ModuleId::new(package_address, ident_str!(module_name).to_owned()),
		ident_str!(function_name).to_owned(),
		ty_args,
		args,
	))
}

/// Send View Request
pub async fn send_view_request(
	aptos_client: &MovementClientFramework,
	package_address: String,
	module_name: String,
	function_name: String,
	type_arguments: Vec<MoveType>,
	arguments: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, anyhow::Error> {
	let view_response = aptos_client
		.rest_client
		.view(
			&ViewRequest {
				function: EntryFunctionId::from_str(&format!(
					"{package_address}::{module_name}::{function_name}"
				))?,
				type_arguments,
				arguments,
			},
			Option::None,
		)
		.await?;
	Ok(view_response.inner().clone())
}

pub async fn create_local_account(
	private_key: Ed25519PrivateKey,
	client: &RestClient,
) -> Result<LocalAccount, anyhow::Error> {
	// Derive the public key from the private key
	let account_key = AccountKey::from_private_key(private_key);

	// Get the account address from the public key
	let account_address = account_key.authentication_key().account_address();

	// Fetch the current sequence number from the blockchain
	let sequence_number = client.get_account(account_address).await?.inner().sequence_number;

	// Create the LocalAccount
	let local_account = LocalAccount::new(account_address, account_key, sequence_number);

	Ok(local_account)
}
fn keccak256(input: &str) -> Vec<u8> {
	let mut hasher = Keccak::v256();
	let mut output = [0u8; 32];
	hasher.update(input.as_bytes());
	hasher.finalize(&mut output);
	output.to_vec()
}

pub fn to_eip55(address: &str) -> String {
	let lowercased_address = address.trim_start_matches("0x").to_lowercase();
	let hash = keccak256(&lowercased_address);

	lowercased_address
		.chars()
		.enumerate()
		.map(|(i, c)| {
			if c.is_digit(10) {
				c
			} else {
				let byte_index = i / 2;
				let nibble_index = i % 2;
				let hash_byte = hash[byte_index];
				let should_uppercase = (hash_byte >> (4 * (1 - nibble_index))) & 0xF >= 8;

				if should_uppercase {
					c.to_ascii_uppercase()
				} else {
					c.to_ascii_lowercase()
				}
			}
		})
		.collect()
}

pub async fn fund_recipient(recipient: &BridgeAddress<Vec<u8>>) -> Result<(), BridgeContractError> {
	// Parse URLs
	let faucet_url =
		Url::parse(MOVEMENT_FAUCET_URL).map_err(|_| BridgeContractError::InvalidUrl)?;
	let rest_url = Url::parse(MOVEMENT_RPC_URL).map_err(|_| BridgeContractError::InvalidUrl)?;

	// Create clients
	let faucet_client = FaucetClient::new(faucet_url, rest_url);

	// Convert recipient to AccountAddress
	let recipient: [u8; 32] = recipient
		.0
		.clone()
		.try_into()
		.map_err(|_| BridgeContractError::SerializationError)?;
	let account_address = AccountAddress::new(recipient);

	// Execute the funding transaction
	faucet_client
		.fund(account_address, 100_000_000)
		.await
		.map_err(|_| BridgeContractError::FundingError)?;

	Ok(())
}
