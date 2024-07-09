use serde::{Deserialize, Serialize};
use godfig::{
    env_short_default,
    env_or_none
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {

    #[serde(default = "default_mcr_deployment_working_directory")]
    pub mcr_deployment_working_directory : String,

    #[serde(default = "default_mcr_deployment_account_private_key")]
    pub mcr_deployment_account_private_key : String, 
}

env_short_default!(
    default_mcr_deployment_working_directory,
    String,
    "protocol-units/settlement/mcr/contracts"
);

env_short_default!(
    default_mcr_deployment_account_private_key,
	String,
	"0x0"
);

env_or_none!(
	default_maybe_deploy,
	Config,
	default_mcr_deployment_account_private_key,
	default_mcr_deployment_working_directory
);

impl Default for Config {
    fn default() -> Self {
        Config {
            mcr_deployment_working_directory: default_mcr_deployment_working_directory(),
            mcr_deployment_account_private_key: default_mcr_deployment_account_private_key(),
        }
    }
}