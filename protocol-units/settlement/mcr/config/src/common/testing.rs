use godfig::env_short_default;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "Vec::new")]
	pub well_known_account_private_keys: Vec<String>,

	#[serde(default = "default_move_token_contract_address")]
	pub move_token_contract_address: String,

	#[serde(default = "default_movement_staking_contract_address")]
	pub movement_staking_contract_address: String,
}

env_short_default!(default_move_token_contract_address, String, "0x0");

env_short_default!(default_movement_staking_contract_address, String, "0x0");

pub fn maybe_testing() -> Option<Config> {
	std::env::var("MAYBE_TESTING_MCR").ok().map(|_| Config::default())
}

impl Default for Config {
	fn default() -> Self {
		Config {
			well_known_account_private_keys: Vec::new(),
			move_token_contract_address: default_move_token_contract_address(),
			movement_staking_contract_address: default_movement_staking_contract_address(),
		}
	}
}
