use crate::types::AddressError;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::Address;
use alloy::providers::fillers::{
	ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::RootProvider;
use alloy::rlp::{RlpDecodable, RlpEncodable};
use alloy::transports::BoxTransport;
use bridge_util::types::Amount;
use bridge_util::types::BridgeAddress;
use bridge_util::types::BridgeTransferDetails;
use bridge_util::types::BridgeTransferId;
use bridge_util::types::HashLock;
use bridge_util::types::HashLockPreImage;
use bridge_util::types::LockDetails;
use rand::Rng;
use std::hash::{DefaultHasher, Hash, Hasher};

pub const ETH_ADDRESS_LEN: usize = 20;

// Codegen for the MOVE bridge contracts
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	NativeBridgeInitiatorMOVE,
	"abis/NativeBridgeInitiatorMOVE.json"
);
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	NativeBridgeCounterpartyMOVE,
	"abis/NativeBridgeCounterpartyMOVE.json"
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

impl From<HashLock> for EthHash {
	fn from(value: HashLock) -> Self {
		let mut fixed_bytes = [0u8; 32];
		let len = value.0.len().min(32);
		fixed_bytes[..len].copy_from_slice(&value.0[..len]);

		Self(hash_vec_u32(&fixed_bytes))
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

pub type InitiatorContract =
	NativeBridgeInitiatorMOVE::NativeBridgeInitiatorMOVEInstance<BoxTransport, AlloyProvider>;
pub type CounterpartyContract =
	NativeBridgeCounterpartyMOVE::NativeBridgeCounterpartyMOVEInstance<BoxTransport, AlloyProvider>;

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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CompletedDetails<A> {
	pub bridge_transfer_id: BridgeTransferId,
	pub recipient: BridgeAddress<A>,
	pub hash_lock: HashLock,
	pub secret: HashLockPreImage,
	pub amount: Amount,
}

impl<A> CompletedDetails<A>
where
	A: From<Vec<u8>>,
{
	pub fn from_bridge_transfer_details(
		bridge_transfer_details: BridgeTransferDetails<Vec<u8>>,
		secret: HashLockPreImage,
	) -> Self {
		CompletedDetails {
			bridge_transfer_id: bridge_transfer_details.bridge_transfer_id,
			recipient: BridgeAddress(A::from(bridge_transfer_details.recipient.0)),
			hash_lock: bridge_transfer_details.hash_lock,
			secret,
			amount: bridge_transfer_details.amount,
		}
	}

	pub fn from_lock_details(lock_details: LockDetails<A>, secret: HashLockPreImage) -> Self {
		CompletedDetails {
			bridge_transfer_id: lock_details.bridge_transfer_id,
			recipient: lock_details.recipient,
			hash_lock: lock_details.hash_lock,
			secret,
			amount: lock_details.amount,
		}
	}
}
