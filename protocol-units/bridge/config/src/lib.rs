pub mod common;

use serde::{Deserialize, Serialize};

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
}

impl Default for Config {
	fn default() -> Self {
		Config {
			eth: common::eth::EthConfig::default(),
			movement: common::movement::MovementConfig::default(),
			testing: common::testing::TestingConfig::default(),
		}
	}
}
