use std::str::FromStr;

use crate::types::EthAddress;
use alloy::primitives::U256;
use alloy::rlp::{Encodable, RlpEncodable};
use keccak_hash::keccak;
use mcr_settlement_client::send_eth_transaction::{
	InsufficentFunds, SendTransactionErrorRule, UnderPriced, VerifyRule,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EthUtilError {
	#[error("Failed to decode hex string")]
	HexDecodeError,
	#[error("Failed to convert Vec<u8> to EthAddress")]
	LengthError,
	#[error("SendTxError: {0}")]
	SendTxError(#[from] alloy::contract::Error),
	#[error("ReceiptError: {0}")]
	GetReceiptError(String),
}

impl FromStr for EthAddress {
	type Err = EthUtilError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		// Try to convert the string to a Vec<u8>
		let vec = hex::decode(s).map_err(|_| EthUtilError::HexDecodeError)?;
		// Ensure the vector has the correct length
		if vec.len() != 20 {
			return Err(EthUtilError::LengthError);
		}
		// Try to convert the Vec<u8> to EthAddress
		Ok(vec.into())
	}
}

pub fn calculate_storage_slot(key: [u8; 32], mapping_slot: U256) -> U256 {
	#[derive(RlpEncodable)]
	struct SlotKey<'a> {
		key: &'a [u8; 32],
		mapping_slot: U256,
	}

	let slot_key = SlotKey { key: &key, mapping_slot };

	let mut buffer = Vec::new();
	slot_key.encode(&mut buffer);

	let hash = keccak(buffer);
	U256::from_be_slice(&hash.0)
}

pub(crate) fn send_tx_rules() -> Vec<Box<dyn VerifyRule>> {
	let rule1: Box<dyn VerifyRule> = Box::new(SendTransactionErrorRule::<UnderPriced>::new());
	let rule2: Box<dyn VerifyRule> = Box::new(SendTransactionErrorRule::<InsufficentFunds>::new());
	vec![rule1, rule2]
}
