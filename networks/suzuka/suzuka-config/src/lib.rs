use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
	#[error("Could not read the Environment variables")]
	EnvReadError,
	#[error("Could not write the Environment variables")]
	EnvWriteError,
	#[error("Could not write the Bash export string")]
	WriteBashExportStringError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
	pub execution_config: maptos_execution_util::config::Config,
}

impl Config {
	pub fn new(execution_config: maptos_execution_util::config::Config) -> Self {
		Self { execution_config }
	}

	pub fn try_from_env() -> Result<Self, ConfigError> {
		let execution_config = maptos_execution_util::config::Config::try_from_env()
			.map_err(|_| ConfigError::EnvReadError)?;

		Ok(Self { execution_config })
	}

	pub fn write_to_env(&self) -> Result<(), ConfigError> {
		self.execution_config.write_to_env().map_err(|_| ConfigError::EnvWriteError)?;
		Ok(())
	}

	pub fn write_bash_export_string(&self) -> Result<String, ConfigError> {
		Ok(format!(
			"{}",
			self.execution_config
				.write_bash_export_string()
				.map_err(|_| ConfigError::WriteBashExportStringError)?
		))
	}
}

