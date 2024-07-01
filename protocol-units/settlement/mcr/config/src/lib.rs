//! This crate provides configuration parameters for the MCR settlement
//! component of a Movement node.

use serde::{Deserialize, Serialize};

const MCR_CONTRACT_ADDRESS: &str = "0xBf7c7AE15E23B2E19C7a1e3c36e245A71500e181";
const DEFAULT_BATCH_TIMEOUT_MILLIS: u64 = 2000;
const DEFAULT_TX_SEND_RETRIES: u32 = 10;
const DEFAULT_GAS_LIMIT: u64 = 10_000_000_000_000_000;

/// Configuration of the MCR settlement client.
///
/// This structure is meant to be used in serialization of human-readable
/// configuration formats.
/// Validation is done when constructing a client instance; see the
/// mcr-settlement-client crate for details.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	pub rpc_url: Option<String>,
	pub ws_url: Option<String>,
	// TODO: this should be managed in a secrets vault
	pub signer_private_key: Option<String>,
	#[serde(default = "default_mcr_contract_address")]
	pub mcr_contract_address: String,
	#[serde(default = "default_gas_limit")]
	pub gas_limit: u64,
	/// Timeout for batching blocks, in milliseconds
	#[serde(default = "default_batch_timeout")]
	pub batch_timeout: u64,
	#[serde(default = "default_transaction_send_retries")]
	pub transaction_send_retries: u32,
	pub anvil_process_pid: Option<u32>,
}

fn default_mcr_contract_address() -> String {
	MCR_CONTRACT_ADDRESS.into()
}

const fn default_gas_limit() -> u64 {
	DEFAULT_GAS_LIMIT
}

const fn default_transaction_send_retries() -> u32 {
	DEFAULT_TX_SEND_RETRIES
}

const fn default_batch_timeout() -> u64 {
	DEFAULT_BATCH_TIMEOUT_MILLIS
}

impl Default for Config {
	fn default() -> Self {
		Config {
			rpc_url: None,
			ws_url: None,
			signer_private_key: None,
			mcr_contract_address: default_mcr_contract_address(),
			gas_limit: default_gas_limit(),
			batch_timeout: default_batch_timeout(),
			transaction_send_retries: default_transaction_send_retries(),
			anvil_process_pid: None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const EXAMPLE_CONFIG_TOML: &str = r#"
		rpc_url = 'http://localhost:8545'
		ws_url = 'http://localhost:8546'
		signer_private_key = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
	"#;

	#[test]
	fn test_parse_from_toml_with_defaults() -> anyhow::Result<()> {
		let Config {
			rpc_url,
			ws_url,
			signer_private_key,
			mcr_contract_address,
			gas_limit,
			batch_timeout,
			transaction_send_retries,
			anvil_process_pid: _,
		} = toml::from_str(EXAMPLE_CONFIG_TOML)?;
		assert_eq!(rpc_url.unwrap(), "http://localhost:8545");
		assert_eq!(ws_url.unwrap(), "http://localhost:8546");
		assert_eq!(
			signer_private_key.unwrap(),
			"0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
		);
		assert_eq!(mcr_contract_address, MCR_CONTRACT_ADDRESS);
		assert_eq!(gas_limit, DEFAULT_GAS_LIMIT);
		assert_eq!(batch_timeout, DEFAULT_BATCH_TIMEOUT_MILLIS);
		assert_eq!(transaction_send_retries, DEFAULT_TX_SEND_RETRIES);
		Ok(())
	}
}
