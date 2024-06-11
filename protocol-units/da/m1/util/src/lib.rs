use std::path::PathBuf;
use celestia_rpc::Client;
use celestia_types::nmt::Namespace;
use dot_movement::DotMovement;
use serde::{Deserialize, Serialize};
use m1_da_light_node_grpc::*;
use anyhow::Context;

/// The configuration for the m1-da-light-node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
	#[serde(default = "Config::default_memseq_config")]
	pub memseq_config: Option<memseq_util::Config>,

	/// The service address string
	#[serde(default = "Config::default_service_address")]
	pub service_address: Option<String>,

	/// The celestia app path for when that is being orchestrated locally
	/// This does not have a default because if it is needed, a default is generally not appropriate.
	pub celestia_app_path: Option<String>,

	/// The celestia chain id for when that is being orchestrated locally
	/// This does not have a default because if it is needed, a default is generally not appropriate.
	pub celestia_chain_id: Option<String>,

	/// The celestia node path for when that is being orchestrated locally
	/// This does not have a default because if it is needed, a default is generally not appropriate.
	pub celestia_node_path: Option<String>,

	/// The celestia validator address for when that is being orchestrated locally
	/// This does not have a default because if it is needed, a default is generally not appropriate.
	pub celestia_validator_address: Option<String>,

}

impl Default for Config {
	fn default() -> Self {
		Config {
			celestia_node_url: Config::default_celestia_node_url(),
			celestia_auth_token: None,
			celestia_namespace: Config::default_namespace(),
			verification_mode: Config::default_verification_mode(),
			memseq_config: Config::default_memseq_config(),
			service_address: Config::default_service_address(),
			celestia_app_path: None,
			celestia_chain_id: None,
			celestia_node_path: None,
			celestia_validator_address: None,
		}
	}
}

impl Config {

	/// The default Celestia node URL.
	const DEFAULT_CELESTIA_NODE_URL: &'static str = "ws://localhost:26658";
	pub fn default_celestia_node_url() -> Option<String> {
		Some(Self::DEFAULT_CELESTIA_NODE_URL.to_string())
	}

	/// Gets a result for the Celestia node URL member.
	pub fn try_celestia_node_url(&self) -> Result<String, anyhow::Error> {
		self.celestia_node_url.as_ref().ok_or(anyhow::anyhow!("No Celestia node URL provided")).map(|s| s.to_string())
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
	pub fn try_celestia_namespace(&self) -> Result<Namespace, anyhow::Error> {
		self.celestia_namespace.as_ref().ok_or(anyhow::anyhow!("No Celestia namespace provided")).map(|n| n.clone())
	}

	/// The default verification mode.
	const DEFAULT_VERIFICATION_MODE: &'static str = "M_OF_N";
	pub fn default_verification_mode() -> Option<String> {
		Some(Self::DEFAULT_VERIFICATION_MODE.to_string())
	}

	/// Gets verification mode str as a result.
	pub fn try_verification_mode_str(&self) -> Result<String, anyhow::Error> {
		self.verification_mode.as_ref().ok_or(anyhow::anyhow!("No verification mode provided")).map(|s| s.to_string())
	}

	/// Gets a result for the verification mode member.
	pub fn try_verification_mode(&self) -> Result<VerificationMode, anyhow::Error> {
		let verification_mode_str = self.try_verification_mode_str()?;
		Ok(VerificationMode::from_str_name(verification_mode_str.as_str()).ok_or(
			anyhow::anyhow!("Invalid verification mode: {}", verification_mode_str),
		)?)
	}

	/// Gets a result for the auth token member.
	pub fn try_celestia_auth_token(&self) -> Result<&str, anyhow::Error> {
		self.celestia_auth_token.as_deref().ok_or(anyhow::anyhow!("No Celestia auth token provided"))
	}

	/// Produces the default memseq config.
	pub fn default_memseq_config() -> Option<memseq_util::Config> {
		Some(memseq_util::Config {
			sequencer_chain_id: memseq_util::Config::default_sequencer_chain_id(),
			sequencer_database_path: memseq_util::Config::default_sequencer_database_path(),
		})
	}

	/// Gets a result for the memseq config member.
	pub fn try_memseq_config(&self) -> Result<memseq_util::Config, anyhow::Error> {
		self.memseq_config.as_ref().ok_or(anyhow::anyhow!("No memseq config provided")).map(|c| c.clone())
	}

	/// The default service address.
	const DEFAULT_SERVICE_ADDRESS: &'static str = "0.0.0.0:30730";
	pub fn default_service_address() -> Option<String> {
		Some(Self::DEFAULT_SERVICE_ADDRESS.to_string())
	}

	/// Gets a result for the service address member.
	pub fn try_service_address(&self) -> Result<String, anyhow::Error> {
		self.service_address.as_ref().ok_or(anyhow::anyhow!("No service address provided")).map(|s| s.to_string())
	}

	/// Gets a result for the celestia app path member.
	pub fn try_celestia_app_path(&self) -> Result<String, anyhow::Error> {
		self.celestia_app_path.as_ref().ok_or(anyhow::anyhow!("No Celestia app path provided")).map(|s| s.to_string())
	}

	/// Gets a result for the celestia chain id member.
	pub fn try_celestia_chain_id(&self) -> Result<String, anyhow::Error> {
		self.celestia_chain_id.as_ref().ok_or(anyhow::anyhow!("No Celestia chain id provided")).map(|s| s.to_string())
	}

	/// Gets a result for the celestia node path member.
	pub fn try_celestia_node_path(&self) -> Result<String, anyhow::Error> {
		self.celestia_node_path.as_ref().ok_or(anyhow::anyhow!("No Celestia node path provided")).map(|s| s.to_string())
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
			celestia_auth_token: None,
			celestia_node_url: Config::default_celestia_node_url(),
			celestia_namespace: Config::default_namespace(),
			verification_mode: Config::default_verification_mode(),
			memseq_config : Config::default_memseq_config(),
			service_address: Config::default_service_address(),
			celestia_app_path: None,
			celestia_chain_id: None,
			celestia_node_path: None,
			celestia_validator_address: None,
		};

		let temp_directory = tempfile::tempdir()?;
		let path = temp_directory.path().join("config.toml");
		config.try_write_to_toml_file(&path)?;

		let read_config = Config::try_from_toml_file(&path)?;

		assert_eq!(config, read_config);

		Ok(())


	}

}
