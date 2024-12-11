use godfig::env_default;
use serde::{Deserialize, Serialize};

const DEFAULT_REST_LISTENER_HOSTNAME: &str = "0.0.0.0";
const DEFAULT_REST_LISTENER_PORT: u16 = 30884;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
	/// URL for the bridge indexer database
	#[serde(default = "default_database_url")]
	pub indexer_url: String,

	/// Endpoint for the REST service
	#[serde(default = "default_rest_listener_hostname")]
	pub rest_listener_hostname: String,
	#[serde(default = "default_rest_listener_port")]
	pub rest_port: u16,
	#[serde(default = "default_start_indexer_with_relayer")]
	pub start_indexer_with_relayer: bool,
}

impl Default for IndexerConfig {
	fn default() -> Self {
		Self {
			indexer_url: default_database_url(),
			rest_listener_hostname: default_rest_listener_hostname(),
			rest_port: default_rest_listener_port(),
			start_indexer_with_relayer: default_start_indexer_with_relayer(),
		}
	}
}

fn default_database_url() -> String {
	"postgresql://postgres:password@localhost:5432".to_string()
}

env_default!(default_start_indexer_with_relayer, "RELAYER_START_INDEXER", bool, false);

env_default!(
	default_rest_listener_hostname,
	"REST_LISTENER_HOSTNAME",
	String,
	DEFAULT_REST_LISTENER_HOSTNAME.to_string()
);

env_default!(default_rest_listener_port, "REST_LISTENER_PORT", u16, DEFAULT_REST_LISTENER_PORT);
