use std::str::FromStr;

use crate::AlloyProvider;
use alloy::pubsub::PubSubFrontend;
use alloy_primitives::Address;
use alloy_provider::RootProvider;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EthInitiatorError {
	#[error("Failed to initiate bridge transfer")]
	InitiateTransferError,
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
	#[error("Failed to decode hex string")]
	HexDecodeError,
	#[error("Failed to convert Vec<u8> to EthAddress")]
	LengthError,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, RlpEncodable, RlpDecodable)]
pub struct EthAddress(pub Address);

impl From<Vec<u8>> for EthAddress {
	fn from(vec: Vec<u8>) -> Self {
		// Ensure the vector has the correct length
		assert_eq!(vec.len(), 20);

		let mut bytes = [0u8; 20];
		bytes.copy_from_slice(&vec);
		EthAddress(Address(bytes.into()))
	}
}

impl FromStr for EthAddress {
	type Err = EthInitiatorError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		// Try to convert the string to a Vec<u8>
		let vec = hex::decode(s).map_err(|_| EthInitiatorError::HexDecodeError)?;
		// Ensure the vector has the correct length
		if vec.len() != 20 {
			return Err(EthInitiatorError::LengthError);
		}
		// Try to convert the Vec<u8> to EthAddress
		Ok(vec.into())
	}
}

pub(crate) struct ProviderArgs {
	pub rpc_provider: AlloyProvider,
	pub ws_provider: RootProvider<PubSubFrontend>,
	pub initiator_address: EthAddress,
	pub counterparty_address: EthAddress,
	pub gas_limit: u64,
	pub num_tx_send_retries: u32,
	pub chain_id: String,
}

pub fn vec_to_array(vec: Vec<u8>) -> Result<[u8; 32], &'static str> {
	if vec.len() == 32 {
		// Try to convert the Vec<u8> to [u8; 32]
		match vec.try_into() {
			Ok(array) => Ok(array),
			Err(_) => Err("Failed to convert Vec<u8> to [u8; 32]"),
		}
	} else {
		Err("Vec<u8> does not have exactly 32 elements")
	}
}
