use super::common::{
	default_enable_pruning, default_enable_table_info_service, default_genesis_block_hash_hex,
	default_genesis_timestamp_microseconds, default_maptos_chain_id,
	default_maptos_epoch_snapshot_prune_window, default_maptos_ledger_prune_window,
	default_maptos_private_key_signer_identifier, default_maptos_read_only,
	default_maptos_rest_listen_hostname, default_maptos_rest_listen_port,
	default_maptos_state_merkle_prune_window,
};
use aptos_types::chain_id::ChainId;
use movement_signer_loader::identifiers::SignerIdentifier;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn default_known_framework_release_str() -> String {
	match std::env::var("KNOWN_FRAMEWORK_RELEASE") {
		Ok(val) => val,
		// todo: revert to head
		Err(_) => "elsa".to_string(),
	}
}

pub fn default_dont_increase_epoch_until_version() -> u64 {
	match std::env::var("DONT_INCREASE_EPOCH_UNTIL_VERSION") {
		Ok(val) => val.parse().unwrap(),
		Err(_) => 0,
	}
}

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
	#[serde(default = "default_maptos_private_key_signer_identifier")]
	pub maptos_private_key_signer_identifier: SignerIdentifier,

	#[serde(default = "default_maptos_read_only")]
	pub maptos_read_only: bool,

	/// Whether or not to prune
	#[serde(default = "default_enable_pruning")]
	pub enabled_pruning: bool,

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

	/// The genesis timestamp in microseconds
	#[serde(default = "default_genesis_timestamp_microseconds")]
	pub genesis_timestamp_microseconds: u64,

	/// The genesis block hash
	#[serde(default = "default_genesis_block_hash_hex")]
	pub genesis_block_hash_hex: String,

	/// The known framework release
	#[serde(default = "default_known_framework_release_str")]
	pub known_framework_release_str: String,

	/// The version to not increase the epoch until
	#[serde(default = "default_dont_increase_epoch_until_version")]
	pub dont_increase_epoch_until_version: u64,

	/// Enable the table info service for indexer.
	#[serde(default = "default_enable_table_info_service")]
	pub enable_table_info_service: bool,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			maptos_chain_id: default_maptos_chain_id(),
			maptos_rest_listen_hostname: default_maptos_rest_listen_hostname(),
			maptos_rest_listen_port: default_maptos_rest_listen_port(),
			maptos_private_key_signer_identifier: default_maptos_private_key_signer_identifier(),
			maptos_read_only: default_maptos_read_only(),
			enabled_pruning: default_enable_pruning(),
			maptos_ledger_prune_window: default_maptos_ledger_prune_window(),
			maptos_epoch_snapshot_prune_window: default_maptos_epoch_snapshot_prune_window(),
			maptos_state_merkle_prune_window: default_maptos_state_merkle_prune_window(),
			genesis_timestamp_microseconds: default_genesis_timestamp_microseconds(),
			genesis_block_hash_hex: default_genesis_block_hash_hex(),
			maptos_db_path: None,
			known_framework_release_str: default_known_framework_release_str(),
			dont_increase_epoch_until_version: default_dont_increase_epoch_until_version(),
			enable_table_info_service: default_enable_table_info_service(),
		}
	}
}
