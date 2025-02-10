use serde::{Deserialize, Serialize};

/// Name of the node's signing key in the keyring that is used by default.
pub const DEFAULT_KEY_NAME: &str = "movement_celestia";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	/// Name of the node's signing key in the keyring
	pub key_name: String,
}

impl Default for Config {
	fn default() -> Self {
		Self { key_name: DEFAULT_KEY_NAME.into() }
	}
}
