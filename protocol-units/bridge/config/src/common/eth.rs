use crate::common::DEFAULT_REST_CONNECTION_TIMEOUT;
use alloy::signers::local::PrivateKeySigner;
use godfig::env_default;
use godfig::env_short_default;
use serde::{Deserialize, Serialize};
use std::env;

const DEFAULT_ETH_RPC_CONNECTION_HOSTNAME: &str = "localhost";
const DEFAULT_ETH_RPC_CONNECTION_PORT: u16 = 8545;
const DEFAULT_ETH_WS_CONNECTION_HOSTNAME: &str = "localhost";
const DEFAULT_ETH_WS_CONNECTION_PORT: u16 = 8545; // same as RPC
const DEFAULT_ETH_INITIATOR_CONTRACT: &str = "Oxeee";
const DEFAULT_ETH_COUNTERPARTY_CONTRACT: &str = "0xccc";
const DEFAULT_ETH_WETH_CONTRACT: &str = "0xe3e3";
const DEFAULT_ETH_MOVETOKEN_CONTRACT: &str = "0xe3e2";
const DEFAULT_ASSET: &str = "MOVE";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthConfig {
	// Connection config.
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
	#[serde(default)]
	// Eth chain config.
	pub eth_chain_id: u64,
	#[serde(default = "default_eth_initiator_contract")]
	pub eth_initiator_contract: String,
	#[serde(default = "default_eth_counterparty_contract")]
	pub eth_counterparty_contract: String,
	#[serde(default = "default_eth_weth_contract")]
	pub eth_weth_contract: String,
	#[serde(default = "default_eth_move_token_contract")]
	pub eth_move_token_contract: String,

	#[serde(default = "default_signer_private_key")]
	pub signer_private_key: String,

	#[serde(default = "default_time_lock_secs")]
	pub time_lock_secs: u64,

	#[serde(default = "default_gas_limit")]
	pub gas_limit: u64,
	#[serde(default = "default_transaction_send_retries")]
	pub transaction_send_retries: u32,

	#[serde(default = "default_asset")]
	pub asset: String,

	#[serde(default = "rest_connection_timeout_secs")]
	pub rest_connection_timeout_secs: u64,
}

env_default!(
	rest_connection_timeout_secs,
	"ETH_REST_CONNECTION_TIMEOUT",
	u64,
	DEFAULT_REST_CONNECTION_TIMEOUT
);

env_default!(
	default_eth_initiator_contract,
	"ETH_INITIATOR_CONTRACT",
	String,
	DEFAULT_ETH_INITIATOR_CONTRACT.to_string()
);

env_default!(
	default_eth_counterparty_contract,
	"ETH_COUNTERPARTY_CONTRACT",
	String,
	DEFAULT_ETH_COUNTERPARTY_CONTRACT.to_string()
);

env_default!(
	default_eth_weth_contract,
	"ETH_WETH_CONTRACT",
	String,
	DEFAULT_ETH_WETH_CONTRACT.to_string()
);
env_default!(
	default_eth_move_token_contract,
	"ETH_MOVETOKEN_CONTRACT",
	String,
	DEFAULT_ETH_MOVETOKEN_CONTRACT.to_string()
);

env_default!(default_asset, "ASSET", String, DEFAULT_ASSET.to_string());

env_short_default!(default_time_lock_secs, u64, 48 * 60 * 60 as u64); //48h by default

env_short_default!(default_gas_limit, u64, 10_000_000_000_000_000 as u64);

env_short_default!(default_transaction_send_retries, u32, 10 as u32);

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

env_default!(default_eth_chain_id, "ETH_CHAIN_ID", u64, 0);

pub fn default_signer_private_key() -> String {
	let random_wallet = PrivateKeySigner::random();
	let random_wallet_string = random_wallet.to_bytes().to_string();
	env::var("ETH_SIGNER_PRIVATE_KEY").unwrap_or(random_wallet_string)
}

// impl EthConfig {
// 	pub fn build_for_test() -> Self {
// 		Config {
// 			rpc_url: "http://localhost:8545".parse().unwrap(),
// 			ws_url: "ws://localhost:8545".parse().unwrap(),
// 			signer_private_key: PrivateKeySigner::random(),
// 			initiator_contract: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
// 			counterparty_contract: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
// 			weth_contract: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
// 			gas_limit: 10_000_000_000,
// 		}
// 	}
// }

impl EthConfig {
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

impl Default for EthConfig {
	fn default() -> Self {
		EthConfig {
			eth_rpc_connection_protocol: default_eth_rpc_connection_protocol(),
			eth_rpc_connection_hostname: default_eth_rpc_connection_hostname(),
			eth_rpc_connection_port: default_eth_rpc_connection_port(),

			eth_ws_connection_protocol: default_eth_ws_connection_protocol(),
			eth_ws_connection_hostname: default_eth_ws_connection_hostname(),
			eth_ws_connection_port: default_eth_ws_connection_port(),
			eth_chain_id: default_eth_chain_id(),

			eth_initiator_contract: default_eth_initiator_contract(),
			eth_counterparty_contract: default_eth_counterparty_contract(),
			eth_weth_contract: default_eth_weth_contract(),
			eth_move_token_contract: default_eth_move_token_contract(),

			time_lock_secs: default_time_lock_secs(),

			signer_private_key: default_signer_private_key(),
			gas_limit: default_gas_limit(),
			transaction_send_retries: default_transaction_send_retries(),

			asset: default_asset(),

			rest_connection_timeout_secs: rest_connection_timeout_secs(),
		}
	}
}
