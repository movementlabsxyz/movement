pub mod da_db;

use serde::{Deserialize, Serialize};

use m1_da_light_node_util::config::M1DaLightNodeConfig;
use maptos_execution_util::config::MaptosConfig;
use mcr_settlement_config::Config as McrConfig;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(flatten)]
	#[serde(default)]
	pub execution_config: MaptosConfig,

	#[serde(flatten)]
	#[serde(default)]
	pub m1_da_light_node: M1DaLightNodeConfig,

	#[serde(default)]
	pub mcr: McrConfig,

	#[serde(default)]
	pub da_db: da_db::Config,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			execution_config: MaptosConfig::default(),
			m1_da_light_node: M1DaLightNodeConfig::default(),
			mcr: McrConfig::default(),
			da_db: da_db::Config::default(),
		}
	}
}
