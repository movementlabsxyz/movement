use super::common::{
	default_maptos_chain_id, default_maptos_private_key, default_maptos_rest_listen_hostname,
	default_maptos_rest_listen_port,
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
			maptos_db_path: None,
		}
	}
}
