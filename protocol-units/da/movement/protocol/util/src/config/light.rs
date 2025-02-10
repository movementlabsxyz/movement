use super::default::default_celestia_light_node_key_name;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	/// Name of the node's signing key in the keyring
	pub key_name: String,
}

impl Default for Config {
	fn default() -> Self {
		Self { key_name: default_celestia_light_node_key_name() }
	}
}
