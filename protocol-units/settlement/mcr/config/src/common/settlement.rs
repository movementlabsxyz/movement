use serde::{Deserialize, Serialize};
use godfig::env_default;
use alloy_signer_wallet::LocalWallet;
use std::env;

const DEFAULT_MCR_CONTRACT_ADDRESS: &str = "0x0";
const DEFAULT_MOVE_TOKEN_CONTRACT_ADDRESS: &str = "0x0";
const DEFAULT_MOVEMENT_STAKING_CONTRACT_ADDRESS: &str = "0x0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_signer_private_key")]
	pub signer_private_key: String,
    #[serde(default = "default_move_token_contract_address")]
	pub move_token_contract_address: String,
	#[serde(default = "default_movement_staking_contract_address")]
	pub movement_staking_contract_address: String,
	#[serde(default = "default_mcr_contract_address")]
	pub mcr_contract_address: String,
}

pub fn default_signer_private_key() -> String {
	let random_wallet = LocalWallet::random();
	let random_wallet_string = random_wallet.to_bytes().to_string();
	env::var("SIGNER_PRIVATE_KEY").unwrap_or(random_wallet_string)
}

env_default!(
	default_mcr_contract_address,
	"MCR_CONTRACT_ADDRESS",
	String,
	DEFAULT_MCR_CONTRACT_ADDRESS.to_string()
);

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