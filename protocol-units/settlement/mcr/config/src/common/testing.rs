use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "Vec::new")]
    pub well_known_account_private_keys : Vec<String>,
}