use crate::types::AddressError;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::Address;
use alloy::providers::fillers::{
	ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::RootProvider;
use alloy::rlp::{RlpDecodable, RlpEncodable};
use alloy::transports::BoxTransport;
use bridge_util::chains::bridge_contracts::BridgeTransferInitiatedDetails;
use bridge_util::types::Amount;
use bridge_util::types::BridgeAddress;
use bridge_util::types::BridgeTransferId;
use bridge_util::types::Nonce;
use rand::Rng;
use std::hash::{DefaultHasher, Hash, Hasher};

pub const ETH_ADDRESS_LEN: usize = 20;

// Codegen for the MOVE bridge contracts
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	NativeBridge,
	"abis/NativeBridge.json"
);

alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MockMOVEToken,
	"abis/MockMOVEToken.json"
);

/// Specifies the kind of asset being transferred,
/// This will associate the client with its respective ABIs
#[derive(Debug, Clone, Default)]
pub enum AssetKind {
	/// This will initialize the client with the WETH Bridge ABIs
	Weth,
	/// This will initialize the client with the MOVE Bridge ABIs
	#[default]
	Move,
}

impl From<String> for AssetKind {
	fn from(asset: String) -> Self {
		match asset.as_str() {
			"WETH" => AssetKind::Weth,
			"MOVE" => AssetKind::Move,
			_ => panic!("Invalid asset kind"),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct EthHash(pub [u8; 32]);

impl EthHash {
	pub fn random() -> Self {
		let mut rng = rand::thread_rng();
		let mut hash = [0u8; 32];
		rng.fill(&mut hash);
		Self(hash)
	}
}

pub fn hash_vec_u32(data: &[u8; 32]) -> [u8; 32] {
	let mut result = [0u8; 32];

	// Split the data into 4 parts and hash each part
	for (i, chunk) in data.chunks(8).enumerate() {
		let mut hasher = DefaultHasher::new();
		chunk.hash(&mut hasher);
		let partial_hash = hasher.finish().to_be_bytes();

		// Copy the 8-byte partial hash into the result
		result[i * 8..(i + 1) * 8].copy_from_slice(&partial_hash);
	}

	result
}

pub fn hash_static_string(pre_image: &'static str) -> [u8; 32] {
	let mut fixed_bytes = [0u8; 32];
	let pre_image_bytes = pre_image.as_bytes();
	let len = pre_image_bytes.len().min(32);
	fixed_bytes[..len].copy_from_slice(&pre_image_bytes[..len]);
	hash_vec_u32(&fixed_bytes)
}

pub type NativeBridgeContract = NativeBridge::NativeBridgeInstance<BoxTransport, AlloyProvider>;

pub type AlloyProvider = FillProvider<
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, RlpEncodable, RlpDecodable)]
pub struct EthAddress(pub Address);

impl From<EthAddress> for Vec<u8> {
	fn from(address: EthAddress) -> Self {
		address.0 .0.to_vec()
	}
}

impl From<BridgeAddress<EthAddress>> for EthAddress {
	fn from(address: BridgeAddress<EthAddress>) -> Self {
		address.0
	}
}

impl std::ops::Deref for EthAddress {
	type Target = Address;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl TryFrom<Vec<u8>> for EthAddress {
	type Error = AddressError;

	fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
		// Ensure the vector has the correct length
		if vec.len() != ETH_ADDRESS_LEN {
			return Err(AddressError::InvalidByteLength(vec.len()));
		}
		let mut bytes = [0u8; ETH_ADDRESS_LEN];
		bytes.copy_from_slice(&vec);
		Ok(bytes.into())
	}
}

impl From<[u8; 32]> for EthAddress {
	fn from(bytes: [u8; 32]) -> Self {
		let mut address_bytes = [0u8; 20];
		address_bytes.copy_from_slice(&bytes[0..20]);
		address_bytes.into()
	}
}
impl From<[u8; 20]> for EthAddress {
	fn from(bytes: [u8; 20]) -> Self {
		EthAddress(Address(bytes.into()))
	}
}
