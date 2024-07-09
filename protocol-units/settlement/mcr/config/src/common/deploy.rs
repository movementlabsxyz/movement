use serde::{Deserialize, Serialize};
use godfig::env_default;

const DEFAULT_MCR_DEPLOYMENT_ACCOUNT_WORKING_DIRECTORY: &str = "protocol-units/settlement/mcr/contracts";
const DEFAULT_MCR_DEPLOYMENT_ACCOUNT_PRIVATE_KEY: &str = "0x0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {

    #[serde(default = "default_mcr_deployment_working_directory")]
    pub mcr_deployment_working_directory : String,

    #[serde(default = "default_mcr_deployment_account_private_key")]
    pub mcr_deployment_account_private_key : String, 
}

env_default!(
    default_mcr_deployment_working_directory,
    "MCR_DEPLOYMENT_WORKING_DIRECTORY",
    String,
    DEFAULT_MCR_DEPLOYMENT_ACCOUNT_WORKING_DIRECTORY.to_string()
);

env_default!(
    default_mcr_deployment_account_private_key,
	"MCR_DEPLOYMENT_ACCOUNT_PRIVATE_KEY",
	String,
	DEFAULT_MCR_DEPLOYMENT_ACCOUNT_PRIVATE_KEY.to_string()
);