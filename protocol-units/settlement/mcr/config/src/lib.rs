//! This crate provides configuration parameters for the MCR settlement
//! component of a Movement node.

use std::env;
use alloy_signer_wallet::LocalWallet;
use serde::{Deserialize, Serialize};
use godfig::env_default;

const DEFAULT_ETH_RPC_CONNECTION_HOSTNAME: &str = "0.0.0.0";
const DEFAULT_ETH_RPC_CONNECTION_PORT: u16 = 8545;
const DEFAULT_ETH_WS_CONNECTION_HOSTNAME: &str = "0.0.0.0";
const DEFAULT_ETH_WS_CONNECTION_PORT: u16 = 8546;
const DEFAULT_MCR_CONTRACT_ADDRESS: &str = "0xBf7c7AE15E23B2E19C7a1e3c36e245A71500e181";
const DEFAULT_MOVE_TOKEN_CONTRACT_ADDRESS: &str = "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984";
const DEFAULT_MOVEMENT_STAKING_CONTRACT_ADDRESS: &str = "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984";
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
	#[serde(default = "default_eth_rpc_connection_protocol")]
	pub eth_rpc_connection_protocol: String,
	#[serde(default = "default_eth_rpc_connection_hostname")]
	pub eth_rpc_connection_hostname: String,
	#[serde(default = "default_eth_rpc_connection_port")]
	pub eth_rpc_connection_port: u16,

	#[serde(default = "default_eth_ws_connection_protocol")]
	pub eth_ws_connection_protocol: String,
	#[serde(default = "default_eth_ws_connection_hostname")]
	pub eth_ws_connection_hostname: String,
	#[serde(default = "default_eth_ws_connection_port")]
	pub eth_ws_connection_port: u16,
	// TODO: this should be managed in a secrets vault
	#[serde(default = "default_signer_private_key")]
	pub signer_private_key: String,
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

impl Config {


	pub fn eth_rpc_connection_url(&self) -> String {
		format!(
			"{}://{}:{}",
			self.eth_rpc_connection_protocol,
			self.eth_rpc_connection_hostname,
			self.eth_rpc_connection_port
		)
	}

	pub fn eth_ws_connection_url(&self) -> String {
		format!(
			"{}://{}:{}",
			self.eth_ws_connection_protocol,
			self.eth_ws_connection_hostname,
			self.eth_ws_connection_port
		)
	}

}

env_default!(
	default_eth_rpc_connection_protocol,
	"ETH_RPC_CONNECTION_PROTOCOL",
	String,
	"http".to_string()
);

env_default!(
	default_eth_rpc_connection_hostname,
	"ETH_RPC_CONNECTION_HOSTNAME",
	String,
	DEFAULT_ETH_RPC_CONNECTION_HOSTNAME.to_string()
);

env_default!(
	default_eth_rpc_connection_port,
	"ETH_RPC_CONNECTION_PORT",
	u16,
	DEFAULT_ETH_RPC_CONNECTION_PORT
);

env_default!(
	default_eth_ws_connection_protocol,
	"ETH_WS_CONNECTION_PROTOCOL",
	String,
	"ws".to_string()
);

env_default!(
	default_eth_ws_connection_hostname,
	"ETH_WS_CONNECTION_HOSTNAME",
	String,
	DEFAULT_ETH_WS_CONNECTION_HOSTNAME.to_string()
);

env_default!(
	default_eth_ws_connection_port,
	"ETH_WS_CONNECTION_PORT",
	u16,
	DEFAULT_ETH_WS_CONNECTION_PORT
);

env_default!(
	default_mcr_contract_address,
	"MCR_CONTRACT_ADDRESS",
	String,
	DEFAULT_MCR_CONTRACT_ADDRESS.to_string()
);

env_default!(
	default_move_token_contract_address,
	"MOVE_TOKEN_CONTRACT_ADDRESS",
	String,
	DEFAULT_MOVE_TOKEN_CONTRACT_ADDRESS.to_string()
);

env_default!(
	default_movement_staking_contract_address,
	"MOVEMENT_STAKING_CONTRACT_ADDRESS",
	String, 
	DEFAULT_MOVEMENT_STAKING_CONTRACT_ADDRESS.to_string()
);

env_default!(
	default_gas_limit,
	"SETTLEMENT_GAS_LIMIT",
	u64,
	DEFAULT_GAS_LIMIT
);

env_default!(
	default_transaction_send_retries,
	"SETTLEMENT_TRANSACTION_SEND_RETRIES",
	u32,
	DEFAULT_TX_SEND_RETRIES
);

env_default!(
	default_batch_timeout,
	"SETTLEMENT_BATCH_TIMEOUT_MILLIS",
	u64,
	DEFAULT_BATCH_TIMEOUT_MILLIS
);

pub fn default_signer_private_key() -> String {
	let random_wallet = LocalWallet::random();
	let random_wallet_string = random_wallet.to_bytes().to_string();
	env::var("SIGNER_PRIVATE_KEY").unwrap_or(random_wallet_string)
}

impl Default for Config {
	fn default() -> Self {
		Config {
			eth_rpc_connection_protocol: default_eth_rpc_connection_protocol(),
			eth_rpc_connection_hostname: default_eth_rpc_connection_hostname(),
			eth_rpc_connection_port: default_eth_rpc_connection_port(),
			eth_ws_connection_protocol: default_eth_ws_connection_protocol(),
			eth_ws_connection_hostname: default_eth_ws_connection_hostname(),
			eth_ws_connection_port: default_eth_ws_connection_port(),
			signer_private_key: default_signer_private_key(),
			mcr_contract_address: default_mcr_contract_address(),
			gas_limit: default_gas_limit(),
			batch_timeout: default_batch_timeout(),
			transaction_send_retries: default_transaction_send_retries(),
			anvil_process_pid: None
		}
	}
}