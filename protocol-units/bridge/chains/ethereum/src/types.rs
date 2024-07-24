use std::str::FromStr;

use alloy_primitives::Address;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

pub type EthHash = [u8; 32];

#[derive(Debug, PartialEq, Eq, Hash, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct EthAddress(pub Address);

impl From<String> for EthAddress {
	fn from(s: String) -> Self {
		EthAddress(Address::parse_checksummed(s, None).expect("Invalid Ethereum address"))
	}
}

impl From<Vec<u8>> for EthAddress {
	fn from(vec: Vec<u8>) -> Self {
		// Ensure the vector has the correct length
		assert_eq!(vec.len(), 20);

		let mut bytes = [0u8; 20];
		bytes.copy_from_slice(&vec);
		EthAddress(Address(bytes.into()))
	}
}
