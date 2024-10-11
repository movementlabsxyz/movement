use alloy::signers::local::PrivateKeySigner;
use godfig::env_short_default;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "mcr_deployment_working_directory")]
	pub mcr_deployment_working_directory: String,

	#[serde(default = "mcr_deployment_account_private_key")]
	pub mcr_deployment_account_private_key: String,
}

env_short_default!(
	mcr_deployment_working_directory,
	String,
	"protocol-units/settlement/mcr/contracts"
);

env_short_default!(
	mcr_deployment_account_private_key,
	String,
	PrivateKeySigner::random().to_bytes().to_string()
);

pub fn maybe_deploy() -> Option<Config> {
	match std::env::var("MAYBE_DEPLOY_MCR") {
		Ok(str_value) => {
			// if it parses as true then we want to deploy under the default config
			let bool_value = str_value.parse::<bool>().unwrap_or(false);

			if bool_value {
				Some(Config::default())
			} else {
				None
			}
		}
		Err(_) => None,
	}
}

impl Default for Config {
	fn default() -> Self {
		Config {
			mcr_deployment_working_directory: mcr_deployment_working_directory(),
			mcr_deployment_account_private_key: mcr_deployment_account_private_key(),
		}
	}
}
