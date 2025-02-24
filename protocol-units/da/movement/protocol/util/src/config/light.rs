use super::default::{default_celestia_light_node_key_name, default_celestia_light_node_store};

use serde::{Deserialize, Serialize};

use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	/// Name of the node's signing key in the keyring
	#[serde(default = "default_celestia_light_node_key_name")]
	pub key_name: String,
	/// Path name of the node store directory
	#[serde(default)]
	pub node_store: Option<PathBuf>,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			key_name: default_celestia_light_node_key_name(),
			node_store: default_celestia_light_node_store(),
		}
	}
}
