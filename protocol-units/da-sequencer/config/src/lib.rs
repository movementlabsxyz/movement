use ed25519_dalek::SigningKey;
use godfig::env_default;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing::info;

pub const DA_SEQUENCER_DIR: &str = "da-sequencer";

fn default_whitelist_relative_path() -> String {
	"da-sequencer/whitelist".to_string()
}

fn default_signing_key() -> SigningKey {
	let mut bytes = [0u8; 32];
	OsRng.fill_bytes(&mut bytes);

	let signing_key = SigningKey::from_bytes(&bytes);
	let verifying_key = signing_key.verifying_key();

	info!("Using batch signing public key: {}", hex::encode(verifying_key.to_bytes()));

	signing_key
}

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

	#[serde(skip_deserializing, skip_serializing, default = "default_signing_key")]
	pub signing_key: SigningKey,
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

impl Default for DaSequencerConfig {
	fn default() -> Self {
		Self {
			grpc_listen_address: default_grpc_listen_address(),
			block_production_interval_millisec: default_block_production_interval_millisec(),
			stream_heartbeat_interval_sec: default_stream_heartbeat_interval_sec(),
			whitelist_relative_path: default_whitelist_relative_path(),
			signing_key: default_signing_key(),
		}
	}
}
