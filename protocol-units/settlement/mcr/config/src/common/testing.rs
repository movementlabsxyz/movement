use serde::{Deserialize, Serialize};
use godfig::env_default;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "Vec::new")]
    pub well_known_account_private_keys : Vec<String>,

    #[serde(default = "default_mcr_testing_admin_account_private_key")]
    pub mcr_testing_admin_account_private_key : String,

}

env_default!(
    default_mcr_testing_admin_account_private_key,
    "MCR_TESTING_ADMIN_ACCOUNT_PRIVATE_KEY",
    String,
    "0x0".to_string()
);