use serde::{Deserialize, Serialize};
use godfig::{
    env_short_default,
    env_or_none
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "Vec::new")]
    pub well_known_account_private_keys : Vec<String>,

    #[serde(default = "default_mcr_testing_admin_account_private_key")]
    pub mcr_testing_admin_account_private_key : String,

    #[serde(default = "default_move_token_contract_address")]
	pub move_token_contract_address: String,

	#[serde(default = "default_movement_staking_contract_address")]
	pub movement_staking_contract_address: String,

}

env_short_default!(
    default_mcr_testing_admin_account_private_key,
    String,
    "0x0"
);

env_short_default!(
	default_move_token_contract_address,
	String,
	"0x0"
);

env_short_default!(
	default_movement_staking_contract_address,
	String, 
	"0x0"
);

env_or_none!(
    default_maybe_testing,
    Config,
    default_mcr_testing_admin_account_private_key,
    default_move_token_contract_address,
    default_movement_staking_contract_address
);

impl Default for Config {
    fn default() -> Self {
        Config {
            well_known_account_private_keys: Vec::new(),
            mcr_testing_admin_account_private_key: default_mcr_testing_admin_account_private_key(),
            move_token_contract_address: default_move_token_contract_address(),
            movement_staking_contract_address: default_movement_staking_contract_address(),
        }
    }
}