use serde::{Deserialize, Serialize};
use super::util::WellKnownAccount;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "Vec::new")]
    pub well_known_accounts: Vec<WellKnownAccount>,
}