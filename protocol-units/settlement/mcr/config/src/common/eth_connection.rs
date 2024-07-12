use serde::{Deserialize, Serialize};
use godfig::env_default;

const DEFAULT_ETH_RPC_CONNECTION_HOSTNAME: &str = "ethereum-holesky-rpc.publicnode.com";
const DEFAULT_ETH_RPC_CONNECTION_PORT: u16 = 443;
const DEFAULT_ETH_WS_CONNECTION_HOSTNAME: &str = "ethereum-holesky-rpc.publicnode.com";
const DEFAULT_ETH_WS_CONNECTION_PORT: u16 = 443; // same as RPC

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

	#[serde(default)]
	pub eth_chain_id: u64,
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
	default_eth_chain_id,
	"ETH_CHAIN_ID",
	u64,
	0
);

impl Default for Config {
	fn default() -> Self {
		Config {
			eth_rpc_connection_protocol: default_eth_rpc_connection_protocol(),
			eth_rpc_connection_hostname: default_eth_rpc_connection_hostname(),
			eth_rpc_connection_port: default_eth_rpc_connection_port(),

			eth_ws_connection_protocol: default_eth_ws_connection_protocol(),
			eth_ws_connection_hostname: default_eth_ws_connection_hostname(),
			eth_ws_connection_port: default_eth_ws_connection_port(),
			eth_chain_id: default_eth_chain_id(),
		}
	}
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