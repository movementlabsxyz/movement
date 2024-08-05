use godfig::env_short_default;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_move_token_contract_address")]
	pub move_token_contract_address: String,
	#[serde(default = "default_movement_staking_contract_address")]
	pub movement_staking_contract_address: String,
}

env_short_default!(default_move_token_contract_address, String, "0x0");

env_short_default!(default_movement_staking_contract_address, String, "0x0");
