use anyhow::Context;
use celestia_rpc::Client;
use serde::{Deserialize, Serialize};

pub mod common;
pub mod local;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Config {
	Local(local::Config),
}

impl Default for Config {
	fn default() -> Self {
		Self::Local(local::Config::default())
	}
}

impl Config {
	/// Connects to a Celestia node using the config
	pub async fn connect_celestia(&self) -> Result<Client, anyhow::Error> {
		match self {
			Config::Local(local) => {
				let celestia_node_url = format!(
					"http://{}:{}",
					local.bridge.celestia_rpc_connection_hostname,
					local.bridge.celestia_rpc_connection_port
				);
				let celestia_auth_token = local.appd.celestia_auth_token.clone().context(
                    "Failed to get Celestia auth token from config. This is required for connecting to Celestia.",
                )?;

				let client = Client::new(&celestia_node_url, Some(&celestia_auth_token))
					.await
					.map_err(|e| {
						anyhow::anyhow!(
							"Failed to connect to Celestia client at {:?}: {}",
							celestia_node_url,
							e
						)
					})?;

				Ok(client)
			}
		}
	}
}

/// The M1 DA Light Node configuration as should be read from file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct M1DaLightNodeConfig {
	#[serde(default)]
	pub m1_da_light_node_config: Config,
}

impl Default for M1DaLightNodeConfig {
	fn default() -> Self {
		Self { m1_da_light_node_config: Config::default() }
	}
}

impl M1DaLightNodeConfig {
	/// Connects to a Celestia node using the config
	pub async fn connect_celestia(&self) -> Result<Client, anyhow::Error> {
		self.m1_da_light_node_config.connect_celestia().await
	}
}
