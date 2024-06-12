pub mod just_aptos {

	use std::path::PathBuf;

	use anyhow::Context;
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
		PrivateKey, Uniform
	};
	use aptos_sdk::types::chain_id::ChainId;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	pub struct Config {

		/// The chain id for the Aptos node
		#[serde(default = "Config::default_chain_id")]
		pub chain_id: Option<ChainId>,

		/// The URL of the Aptos REST server
		#[serde(default = "Config::default_aptos_rest_listen_url")]
		pub aptos_rest_listen_url: Option<String>,

		/// The URL of the Aptos faucet server
		#[serde(default = "Config::default_aptos_faucet_listen_url")]
		pub aptos_faucet_listen_url: Option<String>,

		/// The private key for the Aptos node
		#[serde(default = "Config::default_aptos_private_key")]
		pub aptos_private_key: Option<Ed25519PrivateKey>,

		/// The path to the Aptos database
		#[serde(default = "Config::default_aptos_db_path")]
		pub aptos_db_path: Option<PathBuf>,
	}

	impl Config {

		/// The default chain id for the Aptos node
		pub fn default_chain_id () -> Option<ChainId> {
			Some(ChainId::default())
		}

		/// Gets the chain id for the Aptos node as a result
		pub fn try_chain_id(&self) -> Result<ChainId, anyhow::Error> {
			self.chain_id.clone().context("Chain id not set.")
		}

		/// The default URL of the Aptos REST server
		pub fn default_aptos_rest_listen_url() -> Option<String> {
			Some("0.0.0.0:30731".to_string())
		}

		/// Gets the URL of the Aptos REST server as a result
		pub fn try_aptos_rest_listen_url(&self) -> Result<String, anyhow::Error> {
			self.aptos_rest_listen_url.clone().context("Aptos REST listen URL not set.")
		}

		/// The default URL of the Aptos faucet server
		pub fn default_aptos_faucet_listen_url() -> Option<String> {
			Some("0.0.0.0:30732".to_string())
		}

		/// Gets the URL of the Aptos faucet server as a result
		pub fn try_aptos_faucet_listen_url(&self) -> Result<String, anyhow::Error> {
			self.aptos_faucet_listen_url.clone().context("Aptos faucet listen URL not set.")
		}

		/// The default private key for the Aptos node
		pub fn default_aptos_private_key() -> Option<Ed25519PrivateKey> {
			Some(Ed25519PrivateKey::generate(&mut rand::thread_rng()))
		}

		/// Gets the private key for the Aptos node as a result
		pub fn try_aptos_private_key(&self) -> Result<Ed25519PrivateKey, anyhow::Error> {
			self.aptos_private_key.clone().context("Aptos private key not set.")
		}

		/// Gets the public key for the Aptos node as a result
		pub fn try_aptos_public_key(&self) -> Result<Ed25519PublicKey, anyhow::Error> {
			Ok(self.try_aptos_private_key()?.public_key())
		}

		/// The default path to the Aptos database
		pub fn default_aptos_db_path() -> Option<PathBuf> {
			// generate a tempdir
			// this should work because the dir will be top level of /tmp
			let tempdir = tempfile::tempdir().expect("Failed to create tempdir");
			Some(tempdir.into_path())
		}

		/// Gets the path to the Aptos database as a result
		pub fn try_aptos_db_path(&self) -> Result<PathBuf, anyhow::Error> {
			self.aptos_db_path.clone().context("Aptos db path not set.")
		}

		/// Creates a new config from the given toml file
		pub fn try_from_toml_file(path : PathBuf) -> Result<Self, anyhow::Error> {
			let toml_str = std::fs::read_to_string(path)?;
			let config: Config = toml::from_str(toml_str.as_str())?;
			Ok(config)
		}

	}

	impl Default for Config {
		fn default() -> Self {
			Self {
				chain_id: Config::default_chain_id(),
				aptos_rest_listen_url: Config::default_aptos_rest_listen_url(),
				aptos_faucet_listen_url: Config::default_aptos_faucet_listen_url(),
				aptos_private_key: Config::default_aptos_private_key(),
				aptos_db_path: Config::default_aptos_db_path(),
			}
		}
	}

}

use std::path::PathBuf;
use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The Aptos config
	#[serde(default = "Config::default_aptos_config")]
	pub aptos_config: Option<just_aptos::Config>,

	/// The light node config
	/// We need to flatten this so that the light node runners can use the same config file
	#[serde(flatten)]
	#[serde(default = "Config::default_light_node_config")]
	pub light_node_config: m1_da_light_node_util::Config,
}

impl Config {

	/// The default Aptos config
	pub fn default_aptos_config() -> Option<just_aptos::Config> {
		Some(just_aptos::Config::default())
	}

	/// Gets the Aptos config as a result
	pub fn try_aptos_config(&self) -> Result<just_aptos::Config, anyhow::Error> {
		self.aptos_config.clone().context("Aptos config not set.")
	}

	/// The default light node config
	pub fn default_light_node_config() -> m1_da_light_node_util::Config {
		m1_da_light_node_util::Config::default()
	}

	/// Gets the Config from a toml file
	pub fn try_from_toml_file(path : PathBuf) -> Result<Self, anyhow::Error> {
		let toml_str = std::fs::read_to_string(path)?;
		let config: Config = toml::from_str(toml_str.as_str())?;
		Ok(config)
	}

}

impl Default for Config {
	fn default() -> Self {
		Self {
			aptos_config: Config::default_aptos_config(),
			light_node_config: Config::default_light_node_config(),
		}
	}
}
