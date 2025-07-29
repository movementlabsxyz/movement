use super::common::{default_metrics_server_hostname, default_metrics_server_port};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsConfig {
	#[serde(default = "default_metrics_server_hostname")]
	pub listen_hostname: String,
	#[serde(default = "default_metrics_server_port")]
	pub listen_port: u16,
}

impl Default for MetricsConfig {
	fn default() -> Self {
		Self {
			listen_hostname: default_metrics_server_hostname(),
			listen_port: default_metrics_server_port(),
		}
	}
}
