use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellKnownAccount {
    pub address: String,
    pub name: String,
}