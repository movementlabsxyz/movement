use alloy::signers::local::PrivateKeySigner;
use godfig::env_default;
use movement_signer_loader::identifiers::{local::Local, SignerIdentifier};
use serde::{Deserialize, Serialize};
use std::env;

const DEFAULT_MCR_CONTRACT_ADDRESS: &str = "0x5fc8d32690cc91d4c39d9d3abcbd16989f875707";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	#[serde(default = "default_should_settle")]
	pub should_settle: bool,
	#[serde(default = "default_signer_identifier")]
	pub signer_identifier: SignerIdentifier,
	#[serde(default = "default_mcr_contract_address")]
	pub mcr_contract_address: String,
	#[serde(default = "default_settlement_super_block_size")]
	pub settlement_super_block_size: u64,
	#[serde(default = "default_settlement_admin_mode")]
	pub settlement_admin_mode: bool,
}

pub fn default_signer_identifier() -> SignerIdentifier {
	let random_wallet = PrivateKeySigner::random();
	let private_key_hex_bytes = random_wallet.to_bytes().to_string();
	let signer_identifier = SignerIdentifier::Local(Local { private_key_hex_bytes });
	signer_identifier
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
			signer_identifier: default_signer_identifier(),
			mcr_contract_address: default_mcr_contract_address(),
			settlement_admin_mode: default_settlement_admin_mode(),
			settlement_super_block_size: default_settlement_super_block_size(),
		}
	}
}
