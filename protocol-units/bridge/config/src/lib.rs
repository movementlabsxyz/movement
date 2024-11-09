use serde::{Deserialize, Serialize};

pub mod common;

pub const BRIDGE_CONF_FOLDER: &str = "bridge";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	/// The ETH connection configuration.
	/// This is mandatory for all possible operations.
	#[serde(default)]
	pub eth: common::eth::EthConfig,

	#[serde(default)]
	pub movement: common::movement::MovementConfig,

	/// Optional testing config
	#[serde(default)]
	pub testing: common::testing::TestingConfig,

	/// Database configuration for the bridge indexer
	#[serde(default)]
	pub database: common::db::DatabaseConfig,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			eth: common::eth::EthConfig::default(),
			movement: common::movement::MovementConfig::default(),
			testing: common::testing::TestingConfig::default(),
			database: common::db::DatabaseConfig::default(),
		}
	}
}

pub fn get_config_path(dot_movement: &dot_movement::DotMovement) -> std::path::PathBuf {
	let mut pathbuff = std::path::PathBuf::from(dot_movement.get_path());
	pathbuff.push(BRIDGE_CONF_FOLDER);
	pathbuff
}

impl Config {
	pub fn suzuka() -> Self {
		Config {
			eth: common::eth::EthConfig::default(),
			movement: common::movement::MovementConfig::suzuka_for_test(),
			testing: common::testing::TestingConfig::default(),
			database: common::db::DatabaseConfig::default(),
		}
	}
}
