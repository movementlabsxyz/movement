use super::common::{default_fin_rest_listen_hostname, default_fin_rest_listen_port};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The URL of the Aptos REST server
	#[serde(default = "default_fin_rest_listen_hostname")]
	pub fin_rest_listen_hostname: String,

	/// The port of the Aptos REST server
	#[serde(default = "default_fin_rest_listen_port")]
	pub fin_rest_listen_port: u16,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			fin_rest_listen_hostname: default_fin_rest_listen_hostname(),
			fin_rest_listen_port: default_fin_rest_listen_port(),
		}
	}
}
