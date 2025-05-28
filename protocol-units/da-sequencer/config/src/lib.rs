use ed25519_dalek::VerifyingKey;
use godfig::env_default;
use hex::FromHex;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub const DA_SEQUENCER_DIR: &str = "da-sequencer";

pub fn get_config_path(dot_movement: &dot_movement::DotMovement) -> std::path::PathBuf {
	let mut pathbuff = std::path::PathBuf::from(dot_movement.get_path());
	pathbuff.push(DA_SEQUENCER_DIR);
	pathbuff
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaSequencerConfig {
	#[serde(default = "default_grpc_listen_address")]
	pub grpc_listen_address: SocketAddr,

	#[serde(default = "default_block_production_interval_millisec")]
	pub block_production_interval_millisec: u64,

	#[serde(default = "default_stream_heartbeat_interval_sec")]
	pub stream_heartbeat_interval_sec: u64,

	#[serde(default = "default_whitelist_relative_path")]
	pub whitelist_relative_path: String,

	#[serde(default = "default_db_storage_relative_path")]
	pub db_storage_relative_path: String,

	#[serde(default)]
	pub main_node_verifying_key: Option<String>,

	#[serde(default = "default_healthcheck_bind_port")]
	pub healthcheck_bind_port: u16,
}

impl DaSequencerConfig {
	pub fn get_main_node_verifying_key(&self) -> Result<Option<VerifyingKey>, anyhow::Error> {
		self.main_node_verifying_key
			.clone()
			.map(|str| {
				//remove 0x at the beginning if exist
				let str = str.strip_prefix("0x").unwrap_or(&str);
				<[u8; 32]>::from_hex(str)
					.map_err(|_| anyhow::anyhow!("Invalid main_node_verifying_key hex"))
					.and_then(|hex| {
						VerifyingKey::from_bytes(&hex)
							.map_err(|_| anyhow::anyhow!("Invalid main_node_verifying_key key"))
					})
			})
			.transpose()
	}
}

env_default!(
	default_grpc_listen_address,
	"MOVEMENT_DA_SEQUENCER_GRPC_LISTEN_ADDRESS",
	SocketAddr,
	"0.0.0.0:30730"
		.parse::<SocketAddr>()
		.expect("Bad da sequencer listener address.")
);

env_default!(
	default_block_production_interval_millisec,
	"MOVEMENT_DA_BLOCK_PRODUCTION_INTERVAL_MILLISEC",
	u64,
	500
);
env_default!(
	default_stream_heartbeat_interval_sec,
	"MOVEMENT_DA_STREAM_HEARTBEAT_INTERVAL_MILLISEC",
	u64,
	10
);
env_default!(default_healthcheck_bind_port, "MOVEMENT_DA_HEALTHCHECK_PORT", u16, 30931);

env_default!(
	default_whitelist_relative_path,
	"MOVEMENT_DA_WHITELIST_RELATIVE_PATH",
	String,
	"whitelist.pubkeys".to_string()
);
env_default!(
	default_db_storage_relative_path,
	"MOVEMENT_DA_DB_STORAGE_RELATIVE_PATH",
	String,
	"da-store".to_string()
);

impl Default for DaSequencerConfig {
	fn default() -> Self {
		Self {
			grpc_listen_address: default_grpc_listen_address(),
			block_production_interval_millisec: default_block_production_interval_millisec(),
			stream_heartbeat_interval_sec: default_stream_heartbeat_interval_sec(),
			whitelist_relative_path: default_whitelist_relative_path(),
			db_storage_relative_path: default_db_storage_relative_path(),
			main_node_verifying_key: None,
			healthcheck_bind_port: default_healthcheck_bind_port(),
		}
	}
}
