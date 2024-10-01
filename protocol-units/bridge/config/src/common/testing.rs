use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingConfig {
	#[serde(default = "Vec::new")]
	pub well_known_account_private_keys: Vec<String>,
}

impl Default for TestingConfig {
	fn default() -> Self {
		TestingConfig { well_known_account_private_keys: Vec::new() }
	}
}
