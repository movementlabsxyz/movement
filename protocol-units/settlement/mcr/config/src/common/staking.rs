use serde::{Deserialize, Serialize};
use godfig::env_default;

const DEFAULT_MOVE_TOKEN_CONTRACT_ADDRESS: &str = "0x0";
const DEFAULT_MOVEMENT_STAKING_CONTRACT_ADDRESS: &str = "0x0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_move_token_contract_address")]
	pub move_token_contract_address: String,
	#[serde(default = "default_movement_staking_contract_address")]
	pub movement_staking_contract_address: String,
}

env_default!(
	default_move_token_contract_address,
	"MOVE_TOKEN_CONTRACT_ADDRESS",
	String,
	DEFAULT_MOVE_TOKEN_CONTRACT_ADDRESS.to_string()
);

env_default!(
	default_movement_staking_contract_address,
	"MOVEMENT_STAKING_CONTRACT_ADDRESS",
	String, 
	DEFAULT_MOVEMENT_STAKING_CONTRACT_ADDRESS.to_string()
);