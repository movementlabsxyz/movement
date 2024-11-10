use super::common::{
	default_maptos_chain_id, default_maptos_epoch_snapshot_prune_window,
	default_maptos_ledger_prune_window, default_maptos_private_key, default_maptos_read_only,
	default_maptos_rest_listen_hostname, default_maptos_rest_listen_port,
	default_maptos_state_merkle_prune_window,
};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The chain id for the Aptos node
	#[serde(default = "default_maptos_chain_id")]
	pub maptos_chain_id: ChainId,

	/// The URL of the Aptos REST server
	#[serde(default = "default_maptos_rest_listen_hostname")]
	pub maptos_rest_listen_hostname: String,

	/// The port of the Aptos REST server
	#[serde(default = "default_maptos_rest_listen_port")]
	pub maptos_rest_listen_port: u16,

	/// The private key for the Aptos node
	#[serde(default = "default_maptos_private_key")]
	pub maptos_private_key: Ed25519PrivateKey,

	#[serde(default = "default_maptos_read_only")]
	pub maptos_read_only: bool,

	/// Ledger prune window
	#[serde(default = "default_maptos_ledger_prune_window")]
	pub maptos_ledger_prune_window: u64,

	/// Epoch snapshot prune window
	#[serde(default = "default_maptos_epoch_snapshot_prune_window")]
	pub maptos_epoch_snapshot_prune_window: u64,

	/// State Merkle prune window
	#[serde(default = "default_maptos_state_merkle_prune_window")]
	pub maptos_state_merkle_prune_window: u64,

	/// The path to the Aptos database
	pub maptos_db_path: Option<PathBuf>,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			maptos_chain_id: default_maptos_chain_id(),
			maptos_rest_listen_hostname: default_maptos_rest_listen_hostname(),
			maptos_rest_listen_port: default_maptos_rest_listen_port(),
			maptos_private_key: default_maptos_private_key(),
			maptos_read_only: default_maptos_read_only(),
			maptos_ledger_prune_window: default_maptos_ledger_prune_window(),
			maptos_epoch_snapshot_prune_window: default_maptos_epoch_snapshot_prune_window(),
			maptos_state_merkle_prune_window: default_maptos_state_merkle_prune_window(),
			maptos_db_path: None,
		}
	}
}
