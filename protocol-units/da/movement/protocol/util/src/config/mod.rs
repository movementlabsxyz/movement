pub mod appd;
pub mod bridge;
pub mod da_light_node;
pub mod default;
pub mod digest_store;

use self::default::{default_celestia_force_new_chain, default_da_light_node_is_initial};

use anyhow::Context;
use aptos_account_whitelist::config::Config as WhitelistConfig;
use aptos_types::account_address::AccountAddress;
use celestia_rpc::Client;
use celestia_types::nmt::Namespace;
use memseq_util::Config as MemseqConfig;
use movement_signer::cryptography::{secp256k1::Secp256k1, Curve};
use movement_signer_loader::{Load, LoadedSigner};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::future::Future;
use std::path::PathBuf;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Network {
	Local,
	Arabica,
	Mocha,
	Mainnet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
	pub network: Network,

	/// The appd configuration
	#[serde(default)]
	pub appd: appd::Config,

	/// The bridge configuration
	#[serde(default)]
	pub bridge: bridge::Config,

	/// The movement-celestia-da-light-node configuration
	#[serde(default)]
	pub da_light_node: da_light_node::Config,

	/// Whether to force a new chain
	#[serde(default = "default_celestia_force_new_chain")]
	pub celestia_force_new_chain: bool,

	/// The memseq configuration
	#[serde(default)]
	pub memseq: MemseqConfig,

	#[serde(default = "default_da_light_node_is_initial")]
	pub da_light_node_is_initial: bool,

	/// The access control config
	#[serde(default)]
	pub access_control: WhitelistConfig,

	/// The digest store configuration
	#[serde(default)]
	pub digest_store: digest_store::Config,
}

impl Default for Config {
	fn default() -> Self {
		let network = std::env::var("CELESTIA_NETWORK").map_or_else(
			|_| Network::Local,
			|network| match network.as_str() {
				"mainnet" => Network::Mainnet,
				"arabica" => Network::Arabica,
				"mocha" => Network::Mocha,
				_ => Network::Local,
			},
		);
		Self {
			network,
			appd: appd::Config::default(),
			bridge: bridge::Config::default(),
			da_light_node: da_light_node::Config::default(),
			celestia_force_new_chain: default_celestia_force_new_chain(),
			memseq: MemseqConfig::default(),
			da_light_node_is_initial: default_da_light_node_is_initial(),
			access_control: WhitelistConfig::default(),
			digest_store: digest_store::Config::default(),
		}
	}
}

impl Config {
	/// Connects to a Celestia node using the config
	pub async fn connect_celestia(&self) -> Result<Client, anyhow::Error> {
		let celestia_node_url = self.appd.celestia_websocket_url();
		let celestia_auth_token = self.appd.celestia_auth_token.clone().context(
			"Failed to get Celestia auth token from config. This is required for connecting to Celestia.",
		)?;

		let client =
			Client::new(&celestia_node_url, Some(&celestia_auth_token)).await.map_err(|e| {
				anyhow::anyhow!(
					"Failed to connect to Celestia client at {:?}: {}",
					celestia_node_url,
					e
				)
			})?;

		Ok(client)
	}

	/// Gets the Celestia namespace
	pub fn celestia_namespace(&self) -> Namespace {
		self.appd.celestia_namespace.clone()
	}

	/// Gets M1 DA Light Node connection protocol
	pub fn movement_da_light_node_connection_protocol(&self) -> String {
		self.da_light_node.movement_da_light_node_connection_protocol.clone()
	}

	/// Gets M1 DA Light Node listen hostname
	pub fn movement_da_light_node_listen_hostname(&self) -> String {
		self.da_light_node.movement_da_light_node_listen_hostname.clone()
	}

	/// Gets M1 DA Light Node listen port
	pub fn movement_da_light_node_listen_port(&self) -> u16 {
		self.da_light_node.movement_da_light_node_listen_port
	}

	/// Gets M1 DA Light Node service
	pub fn movement_da_light_node_service(&self) -> String {
		let hostname = self.movement_da_light_node_listen_hostname();
		let port = self.movement_da_light_node_listen_port();
		format!("{}:{}", hostname, port)
	}

	/// Gets M1 DA Light Node connection hostname
	pub fn movement_da_light_node_connection_hostname(&self) -> String {
		self.da_light_node.movement_da_light_node_connection_hostname.clone()
	}

	/// Gets M1 DA Light Node connection port
	pub fn movement_da_light_node_connection_port(&self) -> u16 {
		self.da_light_node.movement_da_light_node_connection_port
	}

	/// Whether to use HTTP/1.1 for the movement-da-light-node service
	pub fn movement_da_light_node_http1(&self) -> bool {
		self.da_light_node.movement_da_light_node_http1
	}

	/// Gets the memseq path
	pub fn try_memseq_path(&self) -> Result<String, anyhow::Error> {
		self.memseq.sequencer_database_path.clone().context(
                "Failed to get memseq path from config. This is required for initializing the memseq database.",
            )
	}

	/// Gets the da signers sec1 keys
	pub fn da_signers_sec1_keys(&self) -> HashSet<String> {
		self.da_light_node.da_signers.public_keys_hex.clone()
	}

	pub fn block_building_parameters(&self) -> (u32, u64) {
		(self.memseq.memseq_max_block_size, self.memseq.memseq_build_time)
	}

	pub fn whitelisted_accounts(&self) -> Result<Option<HashSet<AccountAddress>>, anyhow::Error> {
		self.access_control.whitelisted_accounts()
	}

	pub fn digest_store_db_path(&self) -> PathBuf {
		self.digest_store.digest_store_db_path.clone()
	}
}

pub trait LoadSigner<C>
where
	C: Curve,
{
	/// Gets the da signing key as a string
	fn da_signer(&self) -> impl Future<Output = Result<LoadedSigner<C>, anyhow::Error>> + Send;
}

impl LoadSigner<Secp256k1> for Config {
	async fn da_signer(&self) -> Result<LoadedSigner<Secp256k1>, anyhow::Error> {
		let identifier: Box<dyn Load<Secp256k1> + Send> =
			Box::new(self.da_light_node.da_signers.signer_identifier.clone());
		let signer = identifier
			.load()
			.await
			.map_err(|e| anyhow::anyhow!("failed to load signer: {}", e))?;
		Ok(signer)
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
