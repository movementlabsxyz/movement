pub mod aptos {

	use std::path::PathBuf;

	use anyhow::Context;
	use aptos_crypto::{
		ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
		PrivateKey, Uniform, ValidCryptoMaterialStringExt,
	};
	use aptos_sdk::types::chain_id::ChainId;

	pub mod env_vars {
		pub const CHAIN_ID: &'static str = "MAPTOS_CHAIN_ID";
		pub const OPT_LISTEN_ADDR: &'static str = "MAPTOS_OPT_LISTEN_ADDR";
		pub const FIN_LISTEN_ADDR: &'static str = "MAPTOS_FIN_LISTEN_ADDR";
		pub const FAUCET_LISTEN_ADDR: &'static str = "MAPTOS_FAUCET_LISTEN_ADDR";
		pub const PRIVATE_KEY: &'static str = "MAPTOS_PRIVATE_KEY";
		pub const PUBLIC_KEY: &'static str = "MAPTOS_PUBLIC_KEY";
		pub const DB_PATH: &'static str = "MAPTOS_DB_PATH";
	}

	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct Config {
		pub chain_id: ChainId,
		pub opt_listen_url: String,
		pub fin_listen_url: String,
		pub faucet_listen_url: String,
		pub private_key: Ed25519PrivateKey,
		pub public_key: Ed25519PublicKey,
		pub db_path: PathBuf,
	}

	impl Config {
		pub fn new(
			chain_id: ChainId,
			opt_listen_url: String,
			fin_listen_url: String,
			faucet_listen_url: String,
			private_key: Ed25519PrivateKey,
			public_key: Ed25519PublicKey,
			db_path: PathBuf,
		) -> Self {
			Self {
				chain_id,
				opt_listen_url,
				fin_listen_url,
				faucet_listen_url,
				private_key,
				public_key,
				db_path,
			}
		}

		pub fn try_from_env() -> Result<Self, anyhow::Error> {
			let chain_id = match std::env::var(env_vars::CHAIN_ID) {
				Ok(chain_id) => {
					serde_json::from_str(chain_id.as_str()).context("Failed to parse chain id")?
				}
				Err(_) => ChainId::default(),
			};

			let opt_listen_url =
				std::env::var(env_vars::OPT_LISTEN_ADDR).unwrap_or("0.0.0.0:30731".to_string());

			let fin_listen_url =
				std::env::var(env_vars::FIN_LISTEN_ADDR).unwrap_or("0.0.0.0:30732".to_string());

			let faucet_listen_url =
				std::env::var(env_vars::FAUCET_LISTEN_ADDR).unwrap_or("0.0.0.0:30733".to_string());

			let private_key = match std::env::var(env_vars::PRIVATE_KEY) {
				Ok(private_key) => Ed25519PrivateKey::from_encoded_string(private_key.as_str())
					.context("Failed to parse private key")?,
				Err(_) => Ed25519PrivateKey::generate(&mut rand::thread_rng()),
			};

			let public_key = private_key.public_key();

			let db_path = match std::env::var(env_vars::DB_PATH) {
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
				opt_listen_url,
				fin_listen_url,
				faucet_listen_url,
				private_key,
				public_key,
				db_path,
			})
		}

		pub fn write_bash_export_string(&self) -> Result<String, anyhow::Error> {
			let mut out = String::new();
			for (name, val) in [
				(env_vars::CHAIN_ID, &serde_json::to_string(&self.chain_id)?),
				(env_vars::OPT_LISTEN_ADDR, &self.opt_listen_url),
				(env_vars::FIN_LISTEN_ADDR, &self.fin_listen_url),
				(env_vars::FAUCET_LISTEN_ADDR, &self.faucet_listen_url),
				(env_vars::PRIVATE_KEY, &self.private_key.to_encoded_string()?),
				(env_vars::PUBLIC_KEY, &self.public_key.to_encoded_string()?),
			] {
				out.push_str(name);
				out.push('=');
				out.push_str(val);
				out.push('\n');
			}
			Ok(out)
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
	pub aptos: aptos::Config,
	pub light_node: m1_da_light_node_util::Config,
}

impl Config {
	pub fn new(
		aptos_config: aptos::Config,
		light_node_config: m1_da_light_node_util::Config,
	) -> Self {
		Self { aptos: aptos_config, light_node: light_node_config }
	}

	pub fn try_from_env() -> Result<Self, anyhow::Error> {
		let aptos_config = aptos::Config::try_from_env()?;
		let light_node_config = m1_da_light_node_util::Config::try_from_env()?;

		Ok(Self { aptos: aptos_config, light_node: light_node_config })
	}

	pub fn write_bash_export_string(&self) -> Result<String, anyhow::Error> {
		Ok(format!(
			"{}\n{}",
			self.aptos.write_bash_export_string()?,
			self.light_node.write_bash_export_string()?
		))
	}
}
