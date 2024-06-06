pub mod just_aptos {

	use std::path::PathBuf;

	use anyhow::Context;
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
		PrivateKey, Uniform, ValidCryptoMaterialStringExt,
	};
	use aptos_sdk::types::chain_id::ChainId;

	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Config {
		pub chain_id: ChainId,
		pub aptos_rest_listen_url: String,
		pub aptos_faucet_listen_url: String,
		pub aptos_private_key: Ed25519PrivateKey,
		pub aptos_public_key: Ed25519PublicKey,
		pub aptos_db_path: PathBuf,
	}

	impl Config {
		pub const CHAIN_ID_ENV_VAR: &'static str = "MONZA_CHAIN_ID";
		pub const APTOS_REST_LISTEN_ADDR_ENV_VAR: &'static str = "MONZA_APTOS_REST_LISTEN_ADDR";
		pub const APTOS_FAUCET_LISTEN_ADDR_ENV_VAR: &'static str = "MONZA_APTOS_FAUCET_LISTEN_ADDR";
		pub const APTOS_PRIVATE_KEY_ENV_VAR: &'static str = "MONZA_APTOS_PRIVATE_KEY";
		pub const APTOS_PUBLIC_KEY_ENV_VAR: &'static str = "MONZA_APTOS_PUBLIC_KEY";
		pub const APTOS_DB_PATH_ENV_VAR: &'static str = "MONZA_APTOS_DB_PATH";

		pub fn new(
			chain_id: ChainId,
			aptos_rest_listen_url: String,
			aptos_faucet_listen_url: String,
			aptos_private_key: Ed25519PrivateKey,
			aptos_public_key: Ed25519PublicKey,
			aptos_db_path: PathBuf,
		) -> Self {
			Self {
				chain_id,
				aptos_rest_listen_url,
				aptos_faucet_listen_url,
				aptos_private_key,
				aptos_public_key,
				aptos_db_path,
			}
		}

		pub fn try_from_env() -> Result<Self, anyhow::Error> {
			let chain_id = match std::env::var(Self::CHAIN_ID_ENV_VAR) {
				Ok(chain_id) => {
					serde_json::from_str(chain_id.as_str()).context("Failed to parse chain id")?
				}
				Err(_) => ChainId::default(),
			};

			let aptos_rest_listen_url = std::env::var(Self::APTOS_REST_LISTEN_ADDR_ENV_VAR)
				.unwrap_or("0.0.0.0:30731".to_string());

			let aptos_faucet_listen_url = std::env::var(Self::APTOS_FAUCET_LISTEN_ADDR_ENV_VAR)
				.unwrap_or("0.0.0.0:30732".to_string());

			let aptos_private_key = match std::env::var(Self::APTOS_PRIVATE_KEY_ENV_VAR) {
				Ok(private_key) => Ed25519PrivateKey::from_encoded_string(private_key.as_str())
					.context("Failed to parse private key")?,
				Err(_) => Ed25519PrivateKey::generate(&mut rand::thread_rng()),
			};

			let aptos_public_key = aptos_private_key.public_key();

			let aptos_db_path = match std::env::var(Self::APTOS_DB_PATH_ENV_VAR) {
				Ok(db_path) => PathBuf::from(db_path),
				Err(_) => {
					// generate a tempdir
					// this should work because the dir will be top level of /tmp
					let tempdir = tempfile::tempdir()?;
					tempdir.into_path()
				}
			};

			Ok(Self {
				chain_id,
				aptos_rest_listen_url,
				aptos_faucet_listen_url,
				aptos_private_key,
				aptos_public_key,
				aptos_db_path,
			})
		}

		pub fn write_to_env(&self) -> Result<(), anyhow::Error> {
			std::env::set_var(Self::CHAIN_ID_ENV_VAR, self.chain_id.to_string());
			std::env::set_var(
				Self::APTOS_REST_LISTEN_ADDR_ENV_VAR,
				self.aptos_rest_listen_url.clone(),
			);
			std::env::set_var(
				Self::APTOS_FAUCET_LISTEN_ADDR_ENV_VAR,
				self.aptos_faucet_listen_url.clone(),
			);
			std::env::set_var(
				Self::APTOS_PRIVATE_KEY_ENV_VAR,
				self.aptos_private_key.to_encoded_string()?,
			);
			std::env::set_var(
				Self::APTOS_PUBLIC_KEY_ENV_VAR,
				self.aptos_public_key.to_encoded_string()?,
			);
			Ok(())
		}

		pub fn write_bash_export_string(&self) -> Result<String, anyhow::Error> {
			Ok(format!(
				"export {}={}\nexport {}={}\nexport {}={}\nexport {}={}\nexport {}={}",
				Self::CHAIN_ID_ENV_VAR,
				serde_json::to_string(&self.chain_id)?,
				Self::APTOS_REST_LISTEN_ADDR_ENV_VAR,
				self.aptos_rest_listen_url,
				Self::APTOS_FAUCET_LISTEN_ADDR_ENV_VAR,
				self.aptos_faucet_listen_url,
				Self::APTOS_PRIVATE_KEY_ENV_VAR,
				self.aptos_private_key.to_encoded_string()?,
				Self::APTOS_PUBLIC_KEY_ENV_VAR,
				self.aptos_public_key.to_encoded_string()?
			))
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
	pub aptos_config: just_aptos::Config,
	pub light_node_config: m1_da_light_node_util::Config,
}

impl Config {
	pub fn new(
		aptos_config: just_aptos::Config,
		light_node_config: m1_da_light_node_util::Config,
	) -> Self {
		Self { aptos_config, light_node_config }
	}

	pub fn try_from_env() -> Result<Self, anyhow::Error> {
		let aptos_config = just_aptos::Config::try_from_env()?;
		let light_node_config = m1_da_light_node_util::Config::try_from_env()?;

		Ok(Self { aptos_config, light_node_config })
	}

	pub fn write_to_env(&self) -> Result<(), anyhow::Error> {
		self.aptos_config.write_to_env()?;
		self.light_node_config.write_to_env()?;
		Ok(())
	}

	pub fn write_bash_export_string(&self) -> Result<String, anyhow::Error> {
		Ok(format!(
			"{}\n{}",
			self.aptos_config.write_bash_export_string()?,
			self.light_node_config.write_bash_export_string()?
		))
	}
}
