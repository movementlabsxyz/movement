use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use maptos_execution_util::config::Config as ExecConfig;
use mcr_settlement_client::eth_client::Config as McrConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(flatten)]
	#[serde(default = "Config::default_execution_config")]
	pub execution_config: ExecConfig,

	#[serde(default = "Config::default_mcr_config")]
	pub mcr: McrConfig,
}

impl Config {
	/// The default execution config
	pub fn default_execution_config() -> ExecConfig {
		ExecConfig::default()
	}

	/// The default MCR settlement client config
	pub fn default_mcr_config() -> McrConfig {
		McrConfig::default()
	}

	/// Gets the Config from a toml file
	pub fn try_from_toml_file(path: &PathBuf) -> Result<Self, anyhow::Error> {
		let toml_str = std::fs::read_to_string(path)?;
		let config: Config = toml::from_str(toml_str.as_str())?;
		Ok(config)
	}

	/// Tries to write the Config to a toml file
	pub fn try_write_to_toml_file(&self, path: &PathBuf) -> Result<(), anyhow::Error> {
		let toml = toml::to_string(self)?;
		std::fs::write(path, toml)?;
		Ok(())
	}
}

impl Default for Config {
	fn default() -> Self {
		Self {
			execution_config: Config::default_execution_config(),
			mcr: Config::default_mcr_config(),
		}
	}
}
