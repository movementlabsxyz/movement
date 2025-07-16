use super::common::{default_indexer_processor_auth_token, default_postgres_connection_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_postgres_connection_string")]
	pub postgres_connection_string: String,

	#[serde(default = "default_indexer_processor_auth_token")]
	pub indexer_processor_auth_token: String,

	/// The list of the processor names.
	/// If not provided, all processors will run for backward compatibility.
	/// The list of processors is defined in the indexer side.
	#[serde(default)]
	pub processor_names: Vec<String>,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			postgres_connection_string: default_postgres_connection_string(),
			indexer_processor_auth_token: default_indexer_processor_auth_token(),
			processor_names: vec![],
		}
	}
}
