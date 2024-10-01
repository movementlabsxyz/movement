use godfig::env_default;
use serde::{Deserialize, Serialize};

const DEFAULT_MOVEMENT_NATIVE_ADDRESS: &str = "0xface";
const DEFAULT_MVT_RPC_CONNECTION_HOSTNAME: &str = "localhost";
const DEFAULT_MVT_RPC_CONNECTION_PORT: u16 = 8080;
const DEFAULT_MVT_FAUCET_CONNECTION_HOSTNAME: &str = "localhost";
const DEFAULT_MVT_FAUCET_CONNECTION_PORT: u16 = 8080;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovementConfig {
	#[serde(default = "default_movement_native_address")]
	pub movement_native_address: String,

	#[serde(default = "default_mvt_rpc_connection_protocol")]
	pub mvt_rpc_connection_protocol: String,
	#[serde(default = "default_mvt_rpc_connection_hostname")]
	pub mvt_rpc_connection_hostname: String,
	#[serde(default = "default_mvt_rpc_connection_port")]
	pub mvt_rpc_connection_port: u16,

	#[serde(default = "default_mvt_faucet_connection_protocol")]
	pub mvt_faucet_connection_protocol: String,
	#[serde(default = "default_mvt_faucet_connection_hostname")]
	pub mvt_faucet_connection_hostname: String,
	#[serde(default = "default_mvt_faucet_connection_port")]
	pub mvt_faucet_connection_port: u16,
}

env_default!(
	default_movement_native_address,
	"MOVEMENT_NATIVE_ADDRESS",
	String,
	DEFAULT_MOVEMENT_NATIVE_ADDRESS.to_string()
);

env_default!(
	default_mvt_rpc_connection_protocol,
	"MVT_RPC_CONNECTION_PROTOCOL",
	String,
	"http".to_string()
);

env_default!(
	default_mvt_rpc_connection_hostname,
	"MVT_RPC_CONNECTION_HOSTNAME",
	String,
	DEFAULT_MVT_RPC_CONNECTION_HOSTNAME.to_string()
);

env_default!(
	default_mvt_rpc_connection_port,
	"MVT_RPC_CONNECTION_PORT",
	u16,
	DEFAULT_MVT_RPC_CONNECTION_PORT
);

env_default!(
	default_mvt_faucet_connection_protocol,
	"MVT_FAUCET_CONNECTION_PROTOCOL",
	String,
	"http".to_string()
);

env_default!(
	default_mvt_faucet_connection_hostname,
	"MVT_FAUCET_CONNECTION_HOSTNAME",
	String,
	DEFAULT_MVT_FAUCET_CONNECTION_HOSTNAME.to_string()
);

env_default!(
	default_mvt_faucet_connection_port,
	"MVT_FAUCET_CONNECTION_PORT",
	u16,
	DEFAULT_MVT_FAUCET_CONNECTION_PORT
);

// impl Config {
// 	pub fn build_for_test() -> Self {
// 		let seed = [3u8; 32];
// 		let mut rng = rand::rngs::StdRng::from_seed(seed);

// 		Config {
// 			rpc_url: Some("http://localhost:8080".parse().unwrap()),
// 			ws_url: Some("ws://localhost:8080".parse().unwrap()),
// 			chain_id: 4.to_string(),
// 			signer_private_key: Arc::new(RwLock::new(LocalAccount::generate(&mut rng))),
// 			initiator_contract: None,
// 			gas_limit: 10_000_000_000,
// 		}
// 	}
// }

impl MovementConfig {
	pub fn mvt_rpc_connection_url(&self) -> String {
		format!(
			"{}://{}:{}",
			self.mvt_rpc_connection_protocol,
			self.mvt_rpc_connection_hostname,
			self.mvt_rpc_connection_port
		)
	}

	pub fn mvt_faucet_connection_url(&self) -> String {
		format!(
			"{}://{}:{}",
			self.mvt_faucet_connection_protocol,
			self.mvt_faucet_connection_hostname,
			self.mvt_faucet_connection_port
		)
	}
}

impl Default for MovementConfig {
	fn default() -> Self {
		MovementConfig {
			movement_native_address: default_movement_native_address(),
			mvt_rpc_connection_protocol: default_mvt_rpc_connection_protocol(),
			mvt_rpc_connection_hostname: default_mvt_rpc_connection_hostname(),
			mvt_rpc_connection_port: default_mvt_rpc_connection_port(),
			mvt_faucet_connection_protocol: default_mvt_rpc_connection_protocol(),
			mvt_faucet_connection_hostname: default_mvt_rpc_connection_hostname(),
			mvt_faucet_connection_port: default_mvt_rpc_connection_port(),
		}
	}
}
