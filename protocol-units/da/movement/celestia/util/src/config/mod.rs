use anyhow::Context;
use aptos_types::account_address::AccountAddress;
use celestia_rpc::Client;
use celestia_types::nmt::Namespace;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub mod common;
pub mod local;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Config {
	Local(local::Config),
	Arabica(local::Config),
	Mocha(local::Config),
}

impl Default for Config {
	fn default() -> Self {
		std::env::var("CELESTIA_NETWORK").map_or_else(
			|_| Config::Local(local::Config::default()),
			|network| match network.as_str() {
				"arabica" => Config::Arabica(local::Config::default()),
				"mocha" => Config::Mocha(local::Config::default()),
				_ => Config::Local(local::Config::default()),
			},
		)
	}
}

impl Config {
	/// Connects to a Celestia node using the config
	pub async fn connect_celestia(&self) -> Result<Client, anyhow::Error> {
		match self {
			Config::Local(local) => {
				let celestia_node_url = format!(
					"{}://{}:{}",
					local.appd.celestia_websocket_connection_protocol,
					local.appd.celestia_websocket_connection_hostname,
					local.appd.celestia_websocket_connection_port
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
			Config::Arabica(local) => {
				// arabica is also local for now
				let celestia_node_url = format!(
					"{}://{}:{}",
					local.appd.celestia_websocket_connection_protocol,
					local.appd.celestia_websocket_connection_hostname,
					local.appd.celestia_websocket_connection_port
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
			Config::Mocha(local) => {
				// mocha is also local for now
				let celestia_node_url = format!(
					"{}://{}:{}",
					local.appd.celestia_websocket_connection_protocol,
					local.appd.celestia_websocket_connection_hostname,
					local.appd.celestia_websocket_connection_port
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

	/// Gets the Celestia namespace
	pub fn celestia_namespace(&self) -> Namespace {
		match self {
			Config::Local(local) => local.appd.celestia_namespace.clone(),
			Config::Arabica(local) => local.appd.celestia_namespace.clone(),
			Config::Mocha(local) => local.appd.celestia_namespace.clone(),
		}
	}

	/// Gets M1 DA Light Node connection protocol
	pub fn movement_da_light_node_connection_protocol(&self) -> String {
		match self {
			Config::Local(local) => {
				local.da_light_node.movement_da_light_node_connection_protocol.clone()
			}
			Config::Arabica(local) => {
				local.da_light_node.movement_da_light_node_connection_protocol.clone()
			}
			Config::Mocha(local) => {
				local.da_light_node.movement_da_light_node_connection_protocol.clone()
			}
		}
	}

	/// Gets M1 DA Light Node listen hostname
	pub fn movement_da_light_node_listen_hostname(&self) -> String {
		match self {
			Config::Local(local) => {
				local.da_light_node.movement_da_light_node_listen_hostname.clone()
			}
			Config::Arabica(local) => {
				local.da_light_node.movement_da_light_node_listen_hostname.clone()
			}
			Config::Mocha(local) => {
				local.da_light_node.movement_da_light_node_listen_hostname.clone()
			}
		}
	}

	/// Gets M1 DA Light Node listen port
	pub fn movement_da_light_node_listen_port(&self) -> u16 {
		match self {
			Config::Local(local) => local.da_light_node.movement_da_light_node_listen_port,
			Config::Arabica(local) => local.da_light_node.movement_da_light_node_listen_port,
			Config::Mocha(local) => local.da_light_node.movement_da_light_node_listen_port,
		}
	}

	/// Gets M1 DA Light Node service
	pub fn movement_da_light_node_service(&self) -> String {
		let hostname = self.movement_da_light_node_listen_hostname();
		let port = self.movement_da_light_node_listen_port();
		format!("{}:{}", hostname, port)
	}

	/// Gets M1 DA Light Node connection hostname
	pub fn movement_da_light_node_connection_hostname(&self) -> String {
		match self {
			Config::Local(local) => {
				local.da_light_node.movement_da_light_node_connection_hostname.clone()
			}
			Config::Arabica(local) => {
				local.da_light_node.movement_da_light_node_connection_hostname.clone()
			}
			Config::Mocha(local) => {
				local.da_light_node.movement_da_light_node_connection_hostname.clone()
			}
		}
	}

	/// Gets M1 DA Light Node connection port
	pub fn movement_da_light_node_connection_port(&self) -> u16 {
		match self {
			Config::Local(local) => local.da_light_node.movement_da_light_node_connection_port,
			Config::Arabica(local) => local.da_light_node.movement_da_light_node_connection_port,
			Config::Mocha(local) => local.da_light_node.movement_da_light_node_connection_port,
		}
	}

	/// Whether to use HTTP/1.1 for the movement-da-light-node service
	pub fn movement_da_light_node_http1(&self) -> bool {
		match self {
			Config::Local(local) => local.da_light_node.movement_da_light_node_http1,
			Config::Arabica(local) => local.da_light_node.movement_da_light_node_http1,
			Config::Mocha(local) => local.da_light_node.movement_da_light_node_http1,
		}
	}

	/// Gets the memseq path
	pub fn try_memseq_path(&self) -> Result<String, anyhow::Error> {
		match self {
			Config::Local(local) => local.memseq.sequencer_database_path.clone().context(
                "Failed to get memseq path from config. This is required for initializing the memseq database.",
            ),
			Config::Arabica(local) => local.memseq.sequencer_database_path.clone().context(
				"Failed to get memseq path from config. This is required for initializing the memseq database.",
			),
			Config::Mocha(local) => local.memseq.sequencer_database_path.clone().context(
				"Failed to get memseq path from config. This is required for initializing the memseq database.",
			),
		}
	}

	/// Gets the da signing key as a string
	pub fn da_signing_key(&self) -> String {
		match self {
			Config::Local(local) => local.da_light_node.da_signers.private_key_hex.clone(),
			Config::Arabica(local) => local.da_light_node.da_signers.private_key_hex.clone(),
			Config::Mocha(local) => local.da_light_node.da_signers.private_key_hex.clone(),
		}
	}

	/// Gets the da signers sec1 keys
	pub fn da_signers_sec1_keys(&self) -> HashSet<String> {
		match self {
			Config::Local(local) => local.da_light_node.da_signers.public_keys_hex.clone(),
			Config::Arabica(local) => local.da_light_node.da_signers.public_keys_hex.clone(),
			Config::Mocha(local) => local.da_light_node.da_signers.public_keys_hex.clone(),
		}
	}

	pub fn try_block_building_parameters(&self) -> Result<(u32, u64), anyhow::Error> {
		match self {
			Config::Local(local) => {
				Ok((local.memseq.memseq_max_block_size, local.memseq.memseq_build_time))
			}
			Config::Arabica(local) => {
				Ok((local.memseq.memseq_max_block_size, local.memseq.memseq_build_time))
			}
			Config::Mocha(local) => {
				Ok((local.memseq.memseq_max_block_size, local.memseq.memseq_build_time))
			}
		}
	}

	pub fn whitelisted_accounts(&self) -> Result<Option<HashSet<AccountAddress>>, anyhow::Error> {
		match self {
			Config::Local(local) => local.access_control.whitelisted_accounts(),
			Config::Arabica(local) => local.access_control.whitelisted_accounts(),
			Config::Mocha(local) => local.access_control.whitelisted_accounts(),
		}
	}
}

/// The M1 DA Light Node configuration as should be read from file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CelestiaDaLightNodeConfig {
	#[serde(default)]
	pub celestia_da_light_node_config: Config,
}

impl Default for CelestiaDaLightNodeConfig {
	fn default() -> Self {
		Self { celestia_da_light_node_config: Config::default() }
	}
}

impl CelestiaDaLightNodeConfig {
	/// Connects to a Celestia node using the config
	pub async fn connect_celestia(&self) -> Result<Client, anyhow::Error> {
		self.celestia_da_light_node_config.connect_celestia().await
	}

	/// Gets the Celestia namespace
	pub fn celestia_namespace(&self) -> Namespace {
		self.celestia_da_light_node_config.celestia_namespace()
	}
}
