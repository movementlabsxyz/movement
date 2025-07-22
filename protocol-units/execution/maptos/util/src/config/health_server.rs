use super::common::{default_health_server_hostname, default_health_server_port};
use serde::{Deserialize, Serialize};

// An additional health server to be used by the indexer(or any other service).
// Do not use this with node since it exposes various endpoints to verify the health of the node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_health_server_hostname")]
	pub hostname: String,
	#[serde(default = "default_health_server_port")]
	pub port: u16,
}

impl Default for Config {
	fn default() -> Self {
		Self { hostname: default_health_server_hostname(), port: default_health_server_port() }
	}
}
