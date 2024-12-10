use alloy::signers::local::PrivateKeySigner;
use godfig::env_default;
use serde::{Deserialize, Serialize};
use std::env;

const DEFAULT_MCR_CONTRACT_ADDRESS: &str = "0x5fc8d32690cc91d4c39d9d3abcbd16989f875707";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_should_settle")]
	pub should_settle: bool,
	#[serde(default = "default_signer_private_key")]
	pub signer_private_key: String,
	#[serde(default = "default_mcr_contract_address")]
	pub mcr_contract_address: String,
	#[serde(default = "default_settlement_super_block_size")]
	pub settlement_super_block_size: u64,
	#[serde(default = "default_settlement_admin_mode")]
	pub settlement_admin_mode: bool,
}

pub fn default_signer_private_key() -> String {
	let random_wallet = PrivateKeySigner::random();
	let random_wallet_string = random_wallet.to_bytes().to_string();
	env::var("ETH_SIGNER_PRIVATE_KEY").unwrap_or(random_wallet_string)
}

env_default!(
	default_mcr_contract_address,
	"MCR_CONTRACT_ADDRESS",
	String,
	DEFAULT_MCR_CONTRACT_ADDRESS.to_string()
);

env_default!(default_settlement_admin_mode, "MCR_SETTLEMENT_ADMIN_MODE", bool, false);

env_default!(default_settlement_super_block_size, "MCR_SETTLEMENT_SUPER_BLOCK_SIZE", u64, 1);

pub fn default_should_settle() -> bool {
	env::var("ETH_SIGNER_PRIVATE_KEY").is_ok()
}

impl Default for Config {
	fn default() -> Self {
		Config {
			should_settle: default_should_settle(),
			signer_private_key: default_signer_private_key(),
			mcr_contract_address: default_mcr_contract_address(),
			settlement_admin_mode: default_settlement_admin_mode(),
			settlement_super_block_size: default_settlement_super_block_size(),
		}
	}
}
