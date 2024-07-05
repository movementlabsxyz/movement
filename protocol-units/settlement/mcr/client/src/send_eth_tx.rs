use crate::eth_client::McrEthConnectorError;
use alloy_contract::CallBuilder;
use alloy_contract::CallDecoder;
use alloy_network::Ethereum;
use alloy_provider::Provider;
use alloy_transport::{Transport, TransportError};
use std::marker::PhantomData;

// Define a rule to verify the error generated when a tx is send to determine if:
// * the Tx must me resend with more gas: return Ok(true)
// * a specific error must be return: return Err(McrEthConnectorError::xxx);
// * the rule doesn't apply: return Ok(false)
pub trait VerifyRule: Sync + Send {
	fn verify(&self, error: &alloy_contract::Error) -> Result<bool, McrEthConnectorError>;
}

pub struct SendTxErrorRule<Kind> {
	_kind: PhantomData<Kind>,
}

impl<Kind> SendTxErrorRule<Kind> {
	pub fn new() -> Self {
		SendTxErrorRule { _kind: PhantomData }
	}
}

// Define the current 2 errors managed.
pub struct UnderPriced;
pub struct InsufficentFunds;

impl VerifyRule for SendTxErrorRule<UnderPriced> {
	fn verify(&self, error: &alloy_contract::Error) -> Result<bool, McrEthConnectorError> {
		let alloy_contract::Error::TransportError(TransportError::ErrorResp(payload)) = error
		else {
			return Ok(false);
		};

		if payload.code == -32000 && payload.message.contains("transaction underpriced") {
			Ok(true)
		} else {
			Ok(false)
		}
	}
}

impl VerifyRule for SendTxErrorRule<InsufficentFunds> {
	fn verify(&self, error: &alloy_contract::Error) -> Result<bool, McrEthConnectorError> {
		let alloy_contract::Error::TransportError(TransportError::ErrorResp(payload)) = error
		else {
			return Ok(false);
		};

		if payload.code == -32000 && payload.message.contains("insufficient funds") {
			Err(McrEthConnectorError::InsufficientFunds(payload.message.clone()))
		} else {
			Ok(false)
		}
	}
}

pub async fn send_tx<
	P: Provider<T, Ethereum> + Clone,
	T: Transport + Clone,
	D: CallDecoder + Clone,
>(
	base_call_builder: CallBuilder<T, &&P, D, Ethereum>,
	send_tx_error_rules: &[Box<dyn VerifyRule>],
	nb_retry: u32,
	gas_limit: u128,
) -> Result<(), anyhow::Error> {
	//validate gaz price.
	let mut estimate_gas = base_call_builder.estimate_gas().await?;
	// Add 20% because initial gas estimate are too low.
	estimate_gas += (estimate_gas * 20) / 100;

	// Sending Tx automatically can lead to errors that depend on the state for Eth.
	// It's convenient to manage some of them automatically to avoid to fail commitment Tx.
	// I define a first one but other should be added depending on the test with mainnet.
	for _ in 0..nb_retry {
		let call_builder = base_call_builder.clone().gas(estimate_gas);

		//detect if the gas price doesn't exceed the limit.
		let gas_price = call_builder.provider.get_gas_price().await?;
		let tx_fee_wei = estimate_gas * gas_price;
		if tx_fee_wei > gas_limit {
			return Err(McrEthConnectorError::GasLimitExceed(tx_fee_wei, gas_limit as u128).into());
		}

		//send the Tx and detect send error.
		let pending_tx = match call_builder.send().await {
			Ok(pending_tx) => pending_tx,
			Err(err) => {
				//apply defined rules.
				for rule in send_tx_error_rules {
					// Verify all rules. If one rule return true or an error stop verification.
					// If true retry with more gas else return the error.
					if rule.verify(&err)? {
						//increase gas of 10% and retry
						estimate_gas += (estimate_gas * 10) / 100;
						tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
						continue;
					}
				}

				return Err(McrEthConnectorError::from(err).into());
			}
		};

		match pending_tx.get_receipt().await {
			// Tx execution fail
			Ok(tx_receipt) if !tx_receipt.status() => {
				tracing::debug!(
					"tx_receipt.gas_used: {} / estimate_gas: {estimate_gas}",
					tx_receipt.gas_used
				);
				if tx_receipt.gas_used == estimate_gas {
					tracing::warn!("Send commitment Tx  fail because of insufficient gas, receipt:{tx_receipt:?} ");
					estimate_gas += (estimate_gas * 10) / 100;
					continue;
				} else {
					return Err(McrEthConnectorError::RpcTxExecution(format!(
						"Send commitment Tx fail, abort Tx, receipt:{tx_receipt:?}"
					))
					.into());
				}
			}
			Ok(_) => return Ok(()),
			Err(err) => return Err(McrEthConnectorError::RpcTxExecution(err.to_string()).into()),
		};
	}

	//Max retry exceed
	Err(McrEthConnectorError::RpcTxExecution(
		"Send commitment Tx fail because of exceed max retry".to_string(),
	)
	.into())
}
