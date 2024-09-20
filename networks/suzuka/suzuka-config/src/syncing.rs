use godfig::env_or_none;
use serde::{Deserialize, Serialize};

/// The execution extension configuration.
/// This covers Suzuka configurations that do not configure the Maptos executor, but do configure the way it is used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	/// The number of times to retry a block if it fails to execute.
	#[serde(default = "default_movement_sync")]
	pub movement_sync: Option<String>,
}

impl Default for Config {
	fn default() -> Self {
		Self { movement_sync: default_movement_sync() }
	}
}

pub fn default_movement_sync() -> Option<String> {
	std::env::var("MOVEMENT_SYNC").ok()
}
