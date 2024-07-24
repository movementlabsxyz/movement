use std::str::FromStr;

use alloy::pubsub::PubSubFrontend;
use alloy_contract::{CallBuilder, CallDecoder};
use alloy_network::{Ethereum, EthereumWallet};
use alloy_primitives::Address;
use alloy_provider::{
	fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
	Provider, RootProvider,
};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use alloy_transport::{BoxTransport, Transport};
use bridge_shared::bridge_contracts::BridgeContractInitiatorError;
use mcr_settlement_client::send_eth_transaction::VerifyRule;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct EthAddress(pub Address);

impl From<String> for EthAddress {
	fn from(s: String) -> Self {
		EthAddress(Address::parse_checksummed(s, None).expect("Invalid Ethereum address"))
	}
}

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
	pub signing_address: EthAddress,
	pub initator_contract: EthAddress,
	pub gas_limit: u64,
	pub num_tx_send_retries: u32,
	pub chain_id: String,
}

pub fn vec_to_array(vec: Vec<u8>) -> Result<[u8; 32], BridgeContractInitiatorError> {
	if vec.len() == 32 {
		// Try to convert the Vec<u8> to [u8; 32]
		match vec.try_into() {
			Ok(array) => Ok(array),
			Err(_) => Err(BridgeContractInitiatorError::ParsePreimageError),
		}
	} else {
		Err(BridgeContractInitiatorError::ParsePreimageError)
	}
}

pub async fn send_transaction<
	P: Provider<T, Ethereum> + Clone,
	T: Transport + Clone,
	D: CallDecoder + Clone,
>(
	base_call_builder: CallBuilder<T, &&P, D, Ethereum>,
	send_transaction_error_rules: &[Box<dyn VerifyRule>],
	number_retry: u32,
	gas_limit: u128,
) -> Result<(), anyhow::Error> {
	//validate gas price.
	let mut estimate_gas = base_call_builder.estimate_gas().await?;
	// Add 20% because initial gas estimate are too low.
	estimate_gas += (estimate_gas * 20) / 100;

	// Sending Transaction automatically can lead to errors that depend on the state for Eth.
	// It's convenient to manage some of them automatically to avoid to fail commitment Transaction.
	// I define a first one but other should be added depending on the test with mainnet.
	for _ in 0..number_retry {
		let call_builder = base_call_builder.clone().gas(estimate_gas);

		//detect if the gas price doesn't execeed the limit.
		let gas_price = call_builder.provider.get_gas_price().await?;
		let transaction_fee_wei = estimate_gas * gas_price;
		if transaction_fee_wei > gas_limit {
			return Err(BridgeContractInitiatorError::GasLimitExceededError(
				transaction_fee_wei,
				gas_limit,
			)
			.into());
		}

		//send the Transaction and detect send error.
		let pending_transaction = match call_builder.send().await {
			Ok(pending_transaction) => pending_transaction,
			Err(err) => {
				//apply defined rules.
				for rule in send_transaction_error_rules {
					// Verify all rules. If one rule return true or an error stop verification.
					// If true retry with more gas else return the error.
					if rule.verify(&err)? {
						//increase gas of 10% and retry
						estimate_gas += (estimate_gas * 10) / 100;
						tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
						continue;
					}
				}
				return Err(BridgeContractInitiatorError::GenericError(
					"Unknown Error sending the tx".to_string(),
				)
				.into());
			}
		};

		match pending_transaction.get_receipt().await {
			// Transaction execution fail
			Ok(transaction_receipt) if !transaction_receipt.status() => {
				tracing::debug!(
					"transaction_receipt.gas_used: {} / estimate_gas: {estimate_gas}",
					transaction_receipt.gas_used
				);
				if transaction_receipt.gas_used == estimate_gas {
					tracing::warn!("Send commitment Transaction  fail because of insufficient gas, receipt:{transaction_receipt:?} ");
					estimate_gas += (estimate_gas * 10) / 100;
					continue;
				} else {
					return Err(BridgeContractInitiatorError::RpcTransactionExecutionError(
						format!(
						"Send commitment Transaction fail, abort Transaction, receipt:{transaction_receipt:?}"
					),
					)
					.into());
				}
			}
			Ok(_) => return Ok(()),
			Err(err) => {
				return Err(BridgeContractInitiatorError::GenericError(format!(
					"Error getting receipt: {err:?}"
				))
				.into())
			}
		};
	}

	//Max retry exceed
	Err(BridgeContractInitiatorError::RpcTransactionExecutionError(
		"Send commitment Transaction fail because of exceed max retry".to_string(),
	)
	.into())
}
