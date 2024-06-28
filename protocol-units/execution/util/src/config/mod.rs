pub mod chain;
pub mod client;
pub mod common;
pub mod faucet;
pub mod fin;
pub mod indexer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The chain configuration
	#[serde(default)]
	pub chain: chain::Config,

	/// The indexer configuration
	#[serde(default)]
	pub indexer: indexer::Config,

	/// The client configuration
	#[serde(default)]
	pub client: client::Config,

	/// The faucet configuration
	#[serde(default)]
	pub faucet: faucet::Config,

	/// The fin configuration
	#[serde(default)]
	pub fin: fin::Config,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			chain: chain::Config::default(),
			indexer: indexer::Config::default(),
			client: client::Config::default(),
			faucet: faucet::Config::default(),
			fin: fin::Config::default(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaptosConfig {
	pub maptos_config: Config,
}

impl Default for MaptosConfig {
	fn default() -> Self {
		Self { maptos_config: Config::default() }
	}
}
