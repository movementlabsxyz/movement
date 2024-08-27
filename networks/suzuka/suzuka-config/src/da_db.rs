use godfig::env_default;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_da_db_path")]
	pub da_db_path: String,
}

impl Default for Config {
	fn default() -> Self {
		Self { da_db_path: default_da_db_path() }
	}
}

env_default!(default_da_db_path, "SUZUKA_DA_DB_PATH", String, "suzuka-da-db".to_string());
