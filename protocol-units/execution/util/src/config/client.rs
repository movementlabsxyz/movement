use super::common::{
	default_maptos_faucet_rest_connection_hostname, default_maptos_faucet_rest_connection_port,
	default_maptos_rest_connection_hostname, default_maptos_rest_connection_port,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The URL of the Aptos REST server
	#[serde(default = "default_maptos_rest_connection_hostname")]
	pub maptos_rest_connection_hostname: String,

	/// The port of the Aptos REST server
	#[serde(default = "default_maptos_rest_connection_port")]
	pub maptos_rest_connection_port: u16,

	/// The URL of the Aptos REST server
	#[serde(default = "default_maptos_faucet_rest_connection_hostname")]
	pub maptos_faucet_rest_connection_hostname: String,

	/// The port of the Aptos REST server
	#[serde(default = "default_maptos_faucet_rest_connection_port")]
	pub maptos_faucet_rest_connection_port: u16,
}

impl Config {
	pub fn get_rest_url(&self) -> Result<Url, anyhow::Error> {
		let faucet_url = Url::from_str(&format!(
			"http://{}:{}",
			&self.maptos_rest_connection_hostname, self.maptos_rest_connection_port
		))?;
		Ok(faucet_url)
	}

	pub fn get_faucet_url(&self) -> Result<Url, anyhow::Error> {
		let faucet_url = Url::from_str(&format!(
			"http://{}:{}",
			&self.maptos_faucet_rest_connection_hostname, self.maptos_faucet_rest_connection_port
		))?;
		Ok(faucet_url)
	}
}
impl Default for Config {
	fn default() -> Self {
		Self {
			maptos_rest_connection_hostname: default_maptos_rest_connection_hostname(),
			maptos_rest_connection_port: default_maptos_rest_connection_port(),
			maptos_faucet_rest_connection_hostname: default_maptos_faucet_rest_connection_hostname(
			),
			maptos_faucet_rest_connection_port: default_maptos_faucet_rest_connection_port(),
		}
	}
}
