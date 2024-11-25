pub mod chain;
pub mod client;
pub mod common;
pub mod faucet;
pub mod fin;
pub mod indexer;
pub mod indexer_processor;
pub mod load_shedding;
pub mod mempool;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
	/// The chain configuration
	#[serde(default)]
	pub chain: chain::Config,

	/// The indexer configuration
	#[serde(default)]
	pub indexer: indexer::Config,

	/// The indexer processor configuration
	#[serde(default)]
	pub indexer_processor: indexer_processor::Config,

	/// The client configuration
	#[serde(default)]
	pub client: client::Config,

	/// The faucet configuration
	#[serde(default)]
	pub faucet: faucet::Config,

	/// The fin configuration
	#[serde(default)]
	pub fin: fin::Config,

	/// The load shedding parameters
	#[serde(default)]
	pub load_shedding: load_shedding::Config,

	/// The mempool configuration
	#[serde(default)]
	pub mempool: mempool::Config,

	/// Access control
	#[serde(default)]
	pub access_control: aptos_account_whitelist::config::Config,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			chain: chain::Config::default(),
			indexer: indexer::Config::default(),
			indexer_processor: indexer_processor::Config::default(),
			client: client::Config::default(),
			faucet: faucet::Config::default(),
			fin: fin::Config::default(),
			load_shedding: load_shedding::Config::default(),
			mempool: mempool::Config::default(),
			access_control: aptos_account_whitelist::config::Config::default(),
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
