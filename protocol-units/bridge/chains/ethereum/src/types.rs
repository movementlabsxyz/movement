use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::Address;
use alloy::providers::fillers::{
	ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::RootProvider;
use alloy::rlp::{RlpDecodable, RlpEncodable};
use alloy::transports::BoxTransport;
use serde::{Deserialize, Serialize};

pub type EthHash = [u8; 32];
pub(crate) type AlloyProvider = FillProvider<
	JoinFill<
		JoinFill<
			JoinFill<JoinFill<alloy::providers::Identity, GasFiller>, NonceFiller>,
			ChainIdFiller,
		>,
		WalletFiller<EthereumWallet>,
	>,
	RootProvider<BoxTransport>,
	BoxTransport,
	Ethereum,
>;

#[derive(Debug, PartialEq, Eq, Hash, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct EthAddress(pub Address);

impl std::ops::Deref for EthAddress {
	type Target = Address;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

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

impl From<[u8; 32]> for EthAddress {
	fn from(bytes: [u8; 32]) -> Self {
		let mut address_bytes = [0u8; 20];
		address_bytes.copy_from_slice(&bytes[0..20]);
		EthAddress(Address(address_bytes.into()))
	}
}
