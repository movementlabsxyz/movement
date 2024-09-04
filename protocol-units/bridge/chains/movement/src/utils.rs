use crate::MovementClient;
use anyhow::{Context, Result};
use aptos_sdk::{
	crypto::ed25519::Ed25519Signature,
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
		Client as RestClient, Transaction
	},
	transaction_builder::TransactionFactory,
	types::{
		account_address::AccountAddress,
		chain_id::ChainId,
		transaction::{EntryFunction, SignedTransaction, TransactionPayload},
		LocalAccount,
	},
};
use bridge_shared::bridge_contracts::{BridgeContractCounterpartyError, BridgeContractInitiatorError};
use derive_new::new;
use serde_json::Value;
use tracing::log::{info, debug, error};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

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

impl From<Vec<u8>> for MovementAddress {
	fn from(vec: Vec<u8>) -> Self {
		// Ensure the vector has the correct length
		assert_eq!(vec.len(), AccountAddress::LENGTH);

		let account_address =
			AccountAddress::from_bytes(vec).expect("Invalid byte length for AccountAddress");
		MovementAddress(account_address)
	}
}

impl From<&str> for MovementAddress {
	fn from(s: &str) -> Self {
		let s = s.trim_start_matches("0x");
		let bytes = hex::decode(s).expect("Invalid hex string");
		bytes.into()
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
	info!("Ledger information retrieved: chain_id = {}", state.chain_id);

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

	debug!("Signed TX: {:?}", signed_tx);

	let response = rest_client
		.submit_and_wait(&signed_tx)
		.await
		.map_err(|e| {
			let err_msg = format!("Transaction submission error: {}", e.to_string());
			error!("{}", err_msg); // Log the error in detail
			err_msg
		})?;

	let txn = response.into_inner();
	debug!("Response: {:?}", txn);

	match &txn {
	Transaction::UserTransaction(user_txn) => {
		if !user_txn.info.success {
		return Err(format!(
			"Transaction failed with status: {}",user_txn.info.vm_status));
		}
	},
	_ => return Err("Expected a UserTransaction, but got a different transaction type.".to_string()),
	}

	Ok(txn)
}

pub fn val_as_str(value: Option<&Value>) -> Result<&str, BridgeContractCounterpartyError> {
	value.as_ref().and_then(|v| v.as_str()).ok_or(BridgeContractCounterpartyError::SerializationError)
}

pub fn val_as_u64(value: Option<&Value>) -> Result<u64, BridgeContractCounterpartyError> {
	value
	    .as_ref()
	    .and_then(|v| v.as_u64())
	    .ok_or(BridgeContractCounterpartyError::SerializationError)
}

pub fn val_as_str_initiator(value: Option<&Value>) -> Result<&str, BridgeContractInitiatorError> {
	value.as_ref().and_then(|v| v.as_str()).ok_or(BridgeContractInitiatorError::SerializationError)
}

pub fn val_as_u64_initiator(value: Option<&Value>) -> Result<u64, BridgeContractInitiatorError> {
	value
	    .as_ref()
	    .and_then(|v| v.as_u64())
	    .ok_or(BridgeContractInitiatorError::SerializationError)
}

pub fn serialize_u64(value: &u64) -> Result<Vec<u8>, BridgeContractCounterpartyError> {
	bcs::to_bytes(value).map_err(|_| BridgeContractCounterpartyError::SerializationError)
}
    
pub fn serialize_vec<T: serde::Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, BridgeContractCounterpartyError> {
	bcs::to_bytes(value).map_err(|_| BridgeContractCounterpartyError::SerializationError)
}

pub fn serialize_u64_initiator(value: &u64) -> Result<Vec<u8>, BridgeContractInitiatorError> {
	bcs::to_bytes(value).map_err(|_| BridgeContractInitiatorError::SerializationError)
}

pub fn serialize_address_initiator(address: &AccountAddress) -> Result<Vec<u8>, BridgeContractInitiatorError> {
	bcs::to_bytes(address).map_err(|_| BridgeContractInitiatorError::SerializationError)
}
    
pub fn serialize_vec_initiator<T: serde::Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, BridgeContractInitiatorError> {
	bcs::to_bytes(value).map_err(|_| BridgeContractInitiatorError::SerializationError)
}
 
// This is not used for now, but we may need to use it in later for estimating gas.
pub async fn simulate_aptos_transaction(
	aptos_client: &MovementClient,
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
		Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
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
	aptos_client: &MovementClient,
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
				))
				.unwrap(),
				type_arguments,
				arguments,
			},
			Option::None,
		)
		.await?;
	Ok(view_response.inner().clone())
}
