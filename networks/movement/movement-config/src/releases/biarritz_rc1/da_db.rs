use godfig::env_default;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_da_db_path")]
	pub da_db_path: String,
	#[serde(default = "default_start_sync_height")]
	pub start_sync_height: u64,
	#[serde(default = "default_allow_sync_from_zero")]
	pub allow_sync_from_zero: bool,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			da_db_path: default_da_db_path(),
			start_sync_height: default_start_sync_height(),
			allow_sync_from_zero: default_allow_sync_from_zero(),
		}
	}
}

env_default!(default_da_db_path, "SUZUKA_DA_DB_PATH", String, "movement-da-db".to_string());
env_default!(default_start_sync_height, "MOVEMENT_START_SYNC_HEIGHT", u64, 0);
env_default!(default_allow_sync_from_zero, "MOVEMENT_ALLOW_SYNC_FROM_ZERO", bool, false);
