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
		Client as RestClient,
	},
	transaction_builder::TransactionFactory,
	types::{
		account_address::AccountAddress,
		chain_id::ChainId,
		transaction::{EntryFunction, SignedTransaction, TransactionPayload},
		LocalAccount,
	},
};
use derive_new::new;
use tracing::log::{info, debug};
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
) -> Result<AptosTransaction> {
	info!("Starting send_aptos_transaction");
        debug!("Payload: {:?}", payload);
        debug!("Signer address: {:?}", signer.address());
	let state = rest_client
		.get_ledger_information()
		.await
		.context("Failed in getting chain id")?
		.into_inner();
	info!("Ledger information retrieved: chain_id = {}", state.chain_id);

	let transaction_factory = TransactionFactory::new(ChainId::new(state.chain_id))
		.with_gas_unit_price(100)
		.with_max_gas_amount(GAS_UNIT_LIMIT);
	debug!("Transaction factory created with chain_id = {}", state.chain_id);

	let latest_account_info = rest_client.get_account(signer.address()).await?;
	let account = latest_account_info.into_inner();  
	let latest_sequence_number = account.sequence_number;	

	let raw_tx = transaction_factory
		.payload(payload)
		.sender(signer.address())
		.sequence_number(latest_sequence_number)
		.build();

	let signed_tx = signer.sign_transaction(raw_tx);
	
	debug!("Transaction signed: {:?}", signed_tx);

	let response = rest_client
		.submit_and_wait(&signed_tx)
		.await
		.map_err(|e| {
		debug!("Transaction submission failed: {:?}", e);
		anyhow::anyhow!(e.to_string())
		})?
		.into_inner();

	debug!("Response: {:?}", response);
	
	Ok(response)
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
        // Log the details of the payload being created
        info!("Creating Aptos transaction payload:");
        info!("  Package address: {:?}", package_address);
        info!("  Module name: {}", module_name);
        info!("  Function name: {}", function_name);
        debug!("  Type arguments: {:?}", ty_args);
        debug!("  Arguments: {:?}", args);

        // Create and return the transaction payload
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
