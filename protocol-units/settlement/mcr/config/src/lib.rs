//! This crate provides configuration parameters for the MCR settlement
//! component of a Movement node.

use std::env;
use alloy_signer_wallet::LocalWallet;
use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use godfig::env_default;
use std::ops::{Deref, DerefMut};

const DEFAULT_ETH_RPC_CONNECTION_HOSTNAME: &str = "ethereum-holesky-rpc.publicnode.com";
const DEFAULT_ETH_RPC_CONNECTION_PORT: u16 = 443;
const DEFAULT_ETH_WS_CONNECTION_HOSTNAME: &str = "ethereum-holesky-rpc.publicnode.com";
const DEFAULT_ETH_WS_CONNECTION_PORT: u16 = 443; // same as RPC
const DEFAULT_MCR_CONTRACT_ADDRESS: &str = "0x0";
const DEFAULT_MOVE_TOKEN_CONTRACT_ADDRESS: &str = "0x0";
const DEFAULT_MOVEMENT_STAKING_CONTRACT_ADDRESS: &str = "0x0";
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
pub struct InnerConfig {
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
	#[serde(default = "default_governor_private_key")]
	pub governor_private_key: String,
	#[serde(default = "default_move_token_contract_address")]
	pub move_token_contract_address: String,
	#[serde(default = "default_movement_staking_contract_address")]
	pub movement_staking_contract_address: String,
	#[serde(default = "default_mcr_contract_address")]
	pub mcr_contract_address: String,
	#[serde(default = "default_gas_limit")]
	pub gas_limit: u64,
	/// Timeout for batching blocks, in milliseconds
	#[serde(default = "default_batch_timeout")]
	pub batch_timeout: u64,
	#[serde(default = "default_transaction_send_retries")]
	pub transaction_send_retries: u32,
	#[serde(default = "Vec::new")]
	pub well_known_accounts: Vec<String>,
	#[serde(default = "Vec::new")]
	pub well_known_addresses: Vec<String>,
	#[serde(default = "default_eth_chain_id")]
	pub eth_chain_id: u64,
}

impl InnerConfig {


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

	pub fn try_governor_address (&self) -> Result<Address, anyhow::Error> {
		let governor_wallet : LocalWallet = self.governor_private_key.parse()?;
		Ok(governor_wallet.address())
	}

}

env_default!(
	default_eth_rpc_connection_protocol,
	"ETH_RPC_CONNECTION_PROTOCOL",
	String,
	"https".to_string()
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

pub fn default_governor_private_key() -> String {
	let random_wallet = LocalWallet::random();
	let random_wallet_string = random_wallet.to_bytes().to_string();
	env::var("GOVERNOR_PRIVATE_KEY").unwrap_or(random_wallet_string)
}

env_default!(
	default_eth_chain_id,
	"ETH_CHAIN_ID",
	u64,
	1
);

impl Default for InnerConfig {
	fn default() -> Self {
		InnerConfig {
			eth_rpc_connection_protocol: default_eth_rpc_connection_protocol(),
			eth_rpc_connection_hostname: default_eth_rpc_connection_hostname(),
			eth_rpc_connection_port: default_eth_rpc_connection_port(),
			eth_ws_connection_protocol: default_eth_ws_connection_protocol(),
			eth_ws_connection_hostname: default_eth_ws_connection_hostname(),
			eth_ws_connection_port: default_eth_ws_connection_port(),
			signer_private_key: default_signer_private_key(),
			governor_private_key: default_governor_private_key(),
			mcr_contract_address: default_mcr_contract_address(),
			move_token_contract_address: default_move_token_contract_address(),
			movement_staking_contract_address: default_movement_staking_contract_address(),
			gas_limit: default_gas_limit(),
			batch_timeout: default_batch_timeout(),
			transaction_send_retries: default_transaction_send_retries(),
			well_known_accounts: Vec::new(),
			well_known_addresses: Vec::new(),
			eth_chain_id: default_eth_chain_id(),
		}
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Config {
	Local(InnerConfig),
	DeployRemote(InnerConfig),
}

impl Default for Config {
	fn default() -> Self {
		Config::DeployRemote(InnerConfig::default())
	}
}

impl Deref for Config {
	type Target = InnerConfig;

	fn deref(&self) -> &Self::Target {
		match self {
			Config::Local(inner) => inner,
			Config::DeployRemote(inner) => inner,
		}
	}
}

impl DerefMut for Config {
	fn deref_mut(&mut self) -> &mut Self::Target {
		match self {
			Config::Local(inner) => inner,
			Config::DeployRemote(inner) => inner,
		}
	}
}