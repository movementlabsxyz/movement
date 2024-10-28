use alloy::signers::local::PrivateKeySigner;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingConfig {
	#[serde(default = "Vec::new")]
	pub eth_well_known_account_private_keys: Vec<String>,
}

impl Default for TestingConfig {
	fn default() -> Self {
		TestingConfig { eth_well_known_account_private_keys: Vec::new() }
	}
}

impl TestingConfig {
	pub fn get_initiator_private_key(&self) -> PrivateKeySigner {
		assert!(
			self.eth_well_known_account_private_keys.len() > 3,
			"Testing Eth Anvil private key not inited."
		);
		PrivateKeySigner::from_str(&self.eth_well_known_account_private_keys[2])
			.expect("Testing config initiator private key parsing error")
	}
	pub fn get_recipient_private_key(&self) -> PrivateKeySigner {
		assert!(
			self.eth_well_known_account_private_keys.len() > 3,
			"Testing Eth Anvil private key not inited."
		);
		PrivateKeySigner::from_str(&self.eth_well_known_account_private_keys[3])
			.expect("Testing config initiator private key parsing error")
	}
}
