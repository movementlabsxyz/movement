use crate::chains::ethereum::types::EthAddress;
use alloy::{
	contract::{CallBuilder, CallDecoder},
	network::Ethereum,
	primitives::{Address, U256},
	providers::Provider,
	rlp::{Encodable, RlpEncodable},
	rpc::types::TransactionReceipt,
	transports::Transport,
};
use keccak_hash::keccak;
use mcr_settlement_client::send_eth_transaction::{
	InsufficentFunds, SendTransactionErrorRule, UnderPriced, VerifyRule,
};
use std::str::FromStr;
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
	#[error("RpcTransactionExecution: {0}")]
	GasLimitExceed(u128, u128),
	#[error("RpcTransactionExecution: {0}")]
	RpcTransactionExecution(String),
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
		Ok(vec.try_into().map_err(|_| EthUtilError::HexDecodeError)?)
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

pub fn send_transaction_rules() -> Vec<Box<dyn VerifyRule>> {
	let rule1: Box<dyn VerifyRule> = Box::new(SendTransactionErrorRule::<UnderPriced>::new());
	let rule2: Box<dyn VerifyRule> = Box::new(SendTransactionErrorRule::<InsufficentFunds>::new());
	vec![rule1, rule2]
}

pub async fn send_transaction<
	P: Provider<T, Ethereum> + Clone,
	T: Transport + Clone,
	D: CallDecoder + Clone,
>(
	base_call_builder: CallBuilder<T, &P, D, Ethereum>,
	signer_address: Address,
	send_transaction_error_rules: &[Box<dyn VerifyRule>],
	number_retry: u32,
	gas_limit: u128,
) -> Result<TransactionReceipt, anyhow::Error> {
	println!("base_call_builder: {:?}", base_call_builder);
	println!("Sending transaction with gas limit: {}", gas_limit);

	// set signer address as from for gas_estimation.
	// The gas estimate need to set teh from before calling.
	let base_call_builder = base_call_builder.from(signer_address);
	//validate gas price.
	let mut estimate_gas = base_call_builder.estimate_gas().await?;
	// Add 20% because initial gas estimate are too low.
	estimate_gas += (estimate_gas * 20) / 100;

	println!("estimated_gas: {}", estimate_gas);

	// Sending Transaction automatically can lead to errors that depend on the state for Eth.
	// It's convenient to manage some of them automatically to avoid to fail commitment Transaction.
	// I define a first one but other should be added depending on the test with mainnet.
	for _ in 0..number_retry {
		let call_builder = base_call_builder.clone().gas(estimate_gas);

		tracing::info!("Call: {:?}", call_builder);

		//detect if the gas price doesn't execeed the limit.
		let gas_price = call_builder.provider.get_gas_price().await?;
		let transaction_fee_wei = estimate_gas * gas_price;
		if transaction_fee_wei > gas_limit {
			return Err(EthUtilError::GasLimitExceed(transaction_fee_wei, gas_limit).into());
		}

		println!("Sending transaction with gas: {}", estimate_gas);

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

				return Err(EthUtilError::from(err).into());
			}
		};

		match pending_transaction.get_receipt().await {
			// Transaction execution fail
			Ok(transaction_receipt) if !transaction_receipt.status() => {
				tracing::debug!(
					"transaction_receipt.gas_used: {} / estimate_gas: {estimate_gas}",
					transaction_receipt.gas_used
				);
				// Some valid Tx can abort cause of insufficient gas without consuming all its gas.
				// Define a threshold a little less than estimated gas to detect them.
				let tx_gas_consumption_threshold = estimate_gas - (estimate_gas * 10) / 100;
				if transaction_receipt.gas_used >= tx_gas_consumption_threshold {
					tracing::info!("Send commitment Transaction  fail because of insufficient gas, receipt:{transaction_receipt:?} ");
					estimate_gas += (estimate_gas * 30) / 100;
					continue;
				} else {
					return Err(EthUtilError::RpcTransactionExecution(format!(
						"Send commitment Transaction fail, abort Transaction, receipt:{transaction_receipt:?}"
					))
					.into());
				}
			}
			Ok(receipt) => return Ok(receipt),
			Err(err) => return Err(EthUtilError::RpcTransactionExecution(err.to_string()).into()),
		};
	}

	//Max retry exceed
	Err(EthUtilError::RpcTransactionExecution(
		"Send commitment Transaction fail because of exceed max retry".to_string(),
	)
	.into())
}
