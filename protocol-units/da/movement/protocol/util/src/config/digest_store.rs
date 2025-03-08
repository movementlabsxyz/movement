use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	/// The path to the sequencer database
	#[serde(default = "default_sequencer_database_path")]
	pub digest_store_db_path: PathBuf,
}

pub fn default_sequencer_database_path() -> PathBuf {
	// check if DIGEST_STORE_DB_PATH is set otherwise randomly generate in /tmp
	std::env::var("DIGEST_STORE_DB_PATH").map(PathBuf::from).unwrap_or_else(|_| {
		let mut path = std::env::temp_dir();
		path.push("digest_store_db");
		path
	})
}

impl Default for Config {
	fn default() -> Self {
		Self { digest_store_db_path: default_sequencer_database_path() }
	}
}
