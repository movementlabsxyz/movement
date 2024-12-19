use crate::common::DEFAULT_REST_CONNECTION_TIMEOUT;
use aptos_crypto::{ed25519::Ed25519PrivateKey, Uniform, ValidCryptoMaterialStringExt};
use godfig::env_default;
use serde::{Deserialize, Serialize};

const DEFAULT_MOVEMENT_NATIVE_ADDRESS: &str = "0xface";
const DEFAULT_MVT_RPC_CONNECTION_HOSTNAME: &str = "127.0.0.1";
const DEFAULT_MVT_RPC_CONNECTION_PORT: u16 = 8080;
const DEFAULT_MVT_FAUCET_CONNECTION_HOSTNAME: &str = "127.0.0.1";
const DEFAULT_MVT_FAUCET_CONNECTION_PORT: u16 = 8081;
const DEFAULT_REST_LISTENER_HOSTNAME: &str = "0.0.0.0";
const DEFAULT_GRPC_LISTENER_HOSTNAME: &str = "0.0.0.0";
const DEFAULT_GRPC_LISTENER_PORT: u16 = 50051;
const DEFAULT_REST_LISTENER_PORT: u16 = 30883;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovementConfig {
	#[serde(default = "default_movement_signer_key")]
	pub movement_signer_key: Ed25519PrivateKey,
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

	#[serde(default = "default_mvt_init_network")]
	pub mvt_init_network: String,

	/// Endpoint for the REST service
	#[serde(default = "default_rest_listener_hostname")]
	pub rest_listener_hostname: String,
	#[serde(default = "default_rest_listener_port")]
	pub rest_port: u16,

	// gRPC service connection details
	#[serde(default = "default_grpc_connection_protocol")]
	pub grpc_protocol: String,
	#[serde(default = "default_grpc_listener_hostname")]
	pub grpc_listener_hostname: String,
	#[serde(default = "default_grpc_listener_port")]
	pub grpc_port: u16,
	#[serde(default = "rest_connection_timeout_secs")]
	pub rest_connection_timeout_secs: u64,
}

env_default!(
	rest_connection_timeout_secs,
	"MVT_REST_CONNECTION_TIMEOUT",
	u64,
	DEFAULT_REST_CONNECTION_TIMEOUT
);

// The default private key
pub fn default_movement_signer_key() -> Ed25519PrivateKey {
	match std::env::var("MOVEMENT_SIGNER_KEY") {
		Ok(val) => Ed25519PrivateKey::from_encoded_string(&val).unwrap(),
		Err(_) => Ed25519PrivateKey::generate(&mut rand::thread_rng()),
	}
}

env_default!(
	default_grpc_connection_protocol,
	"GRPC_CONNECTION_PROTOCOL",
	String,
	"http".to_string()
);

env_default!(
	default_grpc_listener_hostname,
	"GRPC_LISTENER_HOSTNAME",
	String,
	DEFAULT_GRPC_LISTENER_HOSTNAME.to_string()
);

env_default!(default_grpc_listener_port, "GRPC_LISTENER_PORT", u16, DEFAULT_GRPC_LISTENER_PORT);

env_default!(
	default_rest_listener_hostname,
	"REST_LISTENER_HOSTNAME",
	String,
	DEFAULT_REST_LISTENER_HOSTNAME.to_string()
);

env_default!(default_rest_listener_port, "REST_LISTENER_PORT", u16, DEFAULT_REST_LISTENER_PORT);

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

env_default!(default_mvt_init_network, "MVT_FAUCET_INIT_NETWORK", String, "local".to_string());

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

	pub fn for_test() -> Self {
		MovementConfig {
			movement_signer_key: Ed25519PrivateKey::from_encoded_string(
				"0x0000000000000000000000000000000000000000000000000000000000000001",
			)
			.unwrap(),
			movement_native_address:
				"0xf90391c81027f03cdea491ed8b36ffaced26b6df208a9b569e5baf2590eb9b16".to_string(),
			mvt_rpc_connection_protocol: default_mvt_rpc_connection_protocol(),
			mvt_rpc_connection_hostname: default_mvt_rpc_connection_hostname(),
			mvt_rpc_connection_port: 30731,
			mvt_faucet_connection_protocol: default_mvt_rpc_connection_protocol(),
			mvt_faucet_connection_hostname: default_mvt_rpc_connection_hostname(),
			mvt_faucet_connection_port: 30732,
			mvt_init_network: default_mvt_init_network(),
			rest_listener_hostname: default_rest_listener_hostname(),
			rest_port: default_rest_listener_port(),
			grpc_protocol: default_grpc_connection_protocol(),
			grpc_listener_hostname: default_grpc_listener_hostname(),
			grpc_port: default_grpc_listener_port(),
			rest_connection_timeout_secs: rest_connection_timeout_secs(),
		}
	}
}

impl Default for MovementConfig {
	fn default() -> Self {
		MovementConfig {
			movement_signer_key: default_movement_signer_key(),
			movement_native_address: default_movement_native_address(),
			mvt_rpc_connection_protocol: default_mvt_rpc_connection_protocol(),
			mvt_rpc_connection_hostname: default_mvt_rpc_connection_hostname(),
			mvt_rpc_connection_port: default_mvt_rpc_connection_port(),
			mvt_faucet_connection_protocol: default_mvt_rpc_connection_protocol(),
			mvt_faucet_connection_hostname: default_mvt_rpc_connection_hostname(),
			mvt_faucet_connection_port: default_mvt_faucet_connection_port(),
			mvt_init_network: default_mvt_init_network(),
			rest_listener_hostname: default_rest_listener_hostname(),
			rest_port: default_rest_listener_port(),
			grpc_protocol: default_grpc_connection_protocol(),
			grpc_listener_hostname: default_grpc_listener_hostname(),
			grpc_port: default_grpc_listener_port(),
			rest_connection_timeout_secs: rest_connection_timeout_secs(),
		}
	}
}
