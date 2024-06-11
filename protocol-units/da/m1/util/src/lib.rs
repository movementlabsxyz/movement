use std::path::PathBuf;
use celestia_rpc::Client;
use celestia_types::nmt::Namespace;
use dot_movement::DotMovementPath;
use serde::{Deserialize, Serialize};
use m1_da_light_node_grpc::*;
use anyhow::Context;

/// The configuration for the m1-da-light-node
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {

	/// The URL of the Celestia node
	#[serde(default = "Config::default_celestia_node_url")]
	pub celestia_node_url: Option<String>,

	/// The auth token for the Celestia node
	pub celestia_auth_token: Option<String>,

	/// The namespace for the Celestia node
	#[serde(default = "Config::default_namespace")]
	pub celestia_namespace: Option<Namespace>,

	/// The verification mode used against data availability
	#[serde(default = "Config::default_verification_mode")]
	pub verification_mode: Option<String>,

	/// The memseq config
	#[serde(default)]
	pub memseq_config: Option<memseq::config::Config>,

}

impl Config {

	/// The default Celestia node URL.
	const DEFAULT_CELESTIA_NODE_URL: &'static str = "ws://localhost:26658";
	pub fn default_celestia_node_url() -> Option<String> {
		Some(Self::DEFAULT_CELESTIA_NODE_URL.to_string())
	}

	/// Gets a result for the Celestia node URL member.
	pub fn try_celestia_node_url(&self) -> Result<&str, anyhow::Error> {
		self.celestia_node_url.as_deref().ok_or(anyhow::anyhow!("No Celestia node URL provided"))
	}

	/// The default namespace bytes.
	const DEFAULT_NAMESPACE_BYTES: &'static str = "a673006fb64aa2e5360d";
	/// Trys to create a default namespace from the default namespace bytes.
	pub fn try_default_namespace() -> Result<Namespace, anyhow::Error> {
		let namespace_bytes = hex::decode(Self::DEFAULT_NAMESPACE_BYTES)
			.map_err(|e| anyhow::anyhow!("Failed to decode default namespace bytes: {}", e))?;
		Namespace::new_v0(&namespace_bytes).context("Failed to create default namespace")
	}
	/// Gets default namespace option.
	pub fn default_namespace() -> Option<Namespace> {
		Self::try_default_namespace().ok()
	}

	/// Gets a result for the namespace member.
	pub fn try_celestia_namespace(&self) -> Result<&Namespace, anyhow::Error> {
		self.celestia_namespace.as_ref().ok_or(anyhow::anyhow!("No namespace provided"))
	}

	/// The default verification mode.
	const DEFAULT_VERIFICATION_MODE: &'static str = "MofN";
	pub fn default_verification_mode() -> Option<String> {
		Some(Self::DEFAULT_VERIFICATION_MODE.to_string())
	}

	/// Gets verification mode str as a result.
	pub fn try_verification_mode_str(&self) -> Result<&str, anyhow::Error> {
		self.verification_mode.as_deref().ok_or(anyhow::anyhow!("No verification mode provided"))
	}

	/// Gets a result for the verification mode member.
	pub fn try_verification_mode(&self) -> Result<VerificationMode, anyhow::Error> {
		let verification_mode_str = self.try_verification_mode_str()?;
		Ok(VerificationMode::from_str_name(verification_mode_str).ok_or(
			anyhow::anyhow!("Invalid verification mode: {}", verification_mode_str),
		)?)
	}

	/// Gets a result for the auth token member.
	pub fn try_celestia_auth_token(&self) -> Result<&str, anyhow::Error> {
		self.celestia_auth_token.as_deref().ok_or(anyhow::anyhow!("No Celestia auth token provided"))
	}

	/// Try to read the location of the config file from the environment and then read the config from the file
	pub fn try_from_env_toml_file() -> Result<Self, anyhow::Error> {
		
		let path = DotMovementPath::try_from_env()?;
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
		let path = DotMovementPath::try_from_env()?;
		self.try_write_to_toml_file(&path.into())
	}

	/// Try to write the config to a TOML file
	pub fn try_write_to_toml_file(&self, path: &PathBuf) -> Result<(), anyhow::Error> {
		let toml = toml::to_string(self).map_err(|e| anyhow::anyhow!("Failed to serialize config to toml: {}", e))?;
		std::fs::write(path, toml).map_err(|e| anyhow::anyhow!("Failed to write config to file: {}", e))?;
		Ok(())
	}

	/// Connects to a Celestia node using the config
	pub async fn connect_celestia(&self) -> Result<Client, anyhow::Error> {

		let celestia_node_url = self.try_celestia_node_url()?.to_string();
		let celestia_auth_token = self.try_celestia_auth_token()?.to_string();

		let client =
			Client::new(
				&celestia_node_url, 
				Some(&celestia_auth_token)
			).await.map_err(|e| {
				anyhow::anyhow!(
					"Failed to connect to Celestia client at {:?}: {}",
					self.celestia_node_url,
					e
				)
			})?;
		
		Ok(client)
	}
}


#[cfg(test)]
pub mod test {
	use super::*;

	#[test]
	fn test_to_and_from_toml_file() -> Result<(), anyhow::Error> {
		
		let config = Config {
			celestia_auth_token: Some("test".to_string()),
			celestia_node_url: Some("test".to_string()),
			celestia_namespace: Some(Namespace::new_v0(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9])?),
			verification_mode: Some("MofN".to_string()),
			memseq_config : Some(memseq::config::Config::default()),
		};

		let temp_directory = tempfile::tempdir()?;
		let path = temp_directory.path().join("config.toml");
		config.try_write_to_toml_file(&path)?;

		let read_config = Config::try_from_toml_file(&path)?;

		assert_eq!(config, read_config);

		Ok(())


	}

}
