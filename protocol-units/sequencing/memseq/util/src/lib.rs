use std::path::PathBuf;
use dot_movement::DotMovement;
use serde::{Deserialize, Serialize};

/// The configuration for the MemSeq sequencer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {

	/// The chain id of the sequencer
	#[serde(default = "Config::default_sequencer_chain_id")]
	pub sequencer_chain_id : Option<String>,

	/// The path to the sequencer database
	#[serde(default = "Config::default_sequencer_database_path")]
	pub sequencer_database_path : Option<String>,

}

impl Default for Config {
	fn default() -> Self {
		Config {
			sequencer_chain_id: Config::default_sequencer_chain_id(),
			sequencer_database_path: Config::default_sequencer_database_path(),
		}
	}
}

impl Config {

	/// The default sequencer chain id.
	const DEFAULT_SEQUENCER_CHAIN_ID: &'static str = "test";
	pub fn default_sequencer_chain_id() -> Option<String> {
		Some(Self::DEFAULT_SEQUENCER_CHAIN_ID.to_string())
	}

	/// Gets a result for the sequencer chain id member.
	pub fn try_sequencer_chain_id(&self) -> Result<&str, anyhow::Error> {
		self.sequencer_chain_id.as_deref().ok_or(anyhow::anyhow!("No sequencer chain id provided"))
	}

	/// The default sequencer database path.
	const DEFAULT_SEQUENCER_DATABASE_PATH: &'static str = "/tmp/sequencer";
	pub fn default_sequencer_database_path() -> Option<String> {
		Some(Self::DEFAULT_SEQUENCER_DATABASE_PATH.to_string())
	}

	/// Gets a result for the sequencer database path member.
	pub fn try_sequencer_database_path(&self) -> Result<String, anyhow::Error> {
		self.sequencer_database_path.clone().ok_or(anyhow::anyhow!("No sequencer database path provided"))
	}

	/// Try to read the location of the config file from the environment and then read the config from the file
	pub fn try_from_env_toml_file() -> Result<Self, anyhow::Error> {
		
		let path = DotMovement::try_from_env()?;
		let config = Self::try_from_toml_file(&path.into())?;
		Ok(config)

	}

	/// Try to read the config from a TOML file
	pub fn try_from_toml_file(path: &PathBuf) -> Result<Self, anyhow::Error> {
		
		let config: Config = toml::from_str(
			&std::fs::read_to_string(path).map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?,
		)
		.map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
		Ok(config)

	}

	/// Try to write the config file to the location specified in the environment
	pub fn try_write_to_env_toml_file(&self) -> Result<(), anyhow::Error> {
		let path = DotMovement::try_from_env()?;
		self.try_write_to_toml_file(&path.into())
	}

	/// Try to write the config to a TOML file
	pub fn try_write_to_toml_file(&self, path: &PathBuf) -> Result<(), anyhow::Error> {
		let toml = toml::to_string(self).map_err(|e| anyhow::anyhow!("Failed to serialize config to toml: {}", e))?;
		std::fs::write(path, toml).map_err(|e| anyhow::anyhow!("Failed to write config to file: {}", e))?;
		Ok(())
	}
	
}


#[cfg(test)]
pub mod test {
	use super::*;

	#[test]
	fn test_to_and_from_toml_file() -> Result<(), anyhow::Error> {
		
		let config = Config {
			sequencer_chain_id: Some("test".to_string()),
			sequencer_database_path: Some("/tmp/sequencer".to_string()),
		};

		let temp_directory = tempfile::tempdir()?;
		let path = temp_directory.path().join("config.toml");
		config.try_write_to_toml_file(&path)?;

		let read_config = Config::try_from_toml_file(&path)?;

		assert_eq!(config, read_config);

		Ok(())


	}

}