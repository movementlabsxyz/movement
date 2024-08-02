use std::str::FromStr;

use crate::types::EthAddress;
use alloy::pubsub::PubSubFrontend;
use alloy_contract::{CallBuilder, CallDecoder};
use alloy_network::{Ethereum, EthereumWallet};
use alloy_primitives::{Address, U256};
use alloy_provider::{
	fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
	Provider, RootProvider,
};
use alloy_rlp::{Encodable, RlpDecodable, RlpEncodable};
use alloy_rpc_types::TransactionReceipt;
use alloy_transport::{BoxTransport, Transport};
use keccak_hash::keccak;
use thiserror::Error;

pub(crate) type AlloyProvider = FillProvider<
	JoinFill<
		JoinFill<
			JoinFill<JoinFill<alloy::providers::Identity, GasFiller>, NonceFiller>,
			ChainIdFiller,
		>,
		WalletFiller<EthereumWallet>,
	>,
	RootProvider<BoxTransport>,
	BoxTransport,
	Ethereum,
>;

#[derive(Debug, Error)]
pub enum EthUtilError {
	#[error("Failed to decode hex string")]
	HexDecodeError,
	#[error("Failed to convert Vec<u8> to EthAddress")]
	LengthError,
	#[error("SendTxError: {0}")]
	SendTxError(#[from] alloy_contract::Error),
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

pub async fn send_transaction<
	P: Provider<T, Ethereum> + Clone,
	T: Transport + Clone,
	D: CallDecoder + Clone,
>(
	contract_call: CallBuilder<T, &&P, D, Ethereum>,
) -> Result<TransactionReceipt, EthUtilError> {
	let pending_transaction = contract_call.send().await?;

	pending_transaction
		.get_receipt()
		.await
		.map_err(|e| EthUtilError::GetReceiptError(e.to_string()))
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
