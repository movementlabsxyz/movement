use bridge_util::types::Amount;
use bridge_util::types::BridgeAddress;
use bridge_util::types::BridgeTransferDetails;
use bridge_util::types::BridgeTransferId;
use bridge_util::types::HashLock;
use bridge_util::types::HashLockPreImage;
use bridge_util::types::LockDetails;
use std::hash::{DefaultHasher, Hash, Hasher};

use alloy::json_abi::Param;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{Address, FixedBytes};
use alloy::providers::fillers::{
	ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::RootProvider;
use alloy::rlp::{RlpDecodable, RlpEncodable};
use alloy::sol_types::SolEvent;
use alloy::transports::BoxTransport;
use rand::Rng;

pub const INITIATOR_INITIATED_SELECT: FixedBytes<32> =
	AtomicBridgeInitiator::BridgeTransferInitiated::SIGNATURE_HASH;
pub const INITIATOR_COMPLETED_SELECT: FixedBytes<32> =
	AtomicBridgeInitiator::BridgeTransferCompleted::SIGNATURE_HASH;
pub const INITIATOR_REFUNDED_SELECT: FixedBytes<32> =
	AtomicBridgeInitiator::BridgeTransferRefunded::SIGNATURE_HASH;
pub const COUNTERPARTY_LOCKED_SELECT: FixedBytes<32> =
	AtomicBridgeCounterparty::BridgeTransferLocked::SIGNATURE_HASH;
pub const COUNTERPARTY_COMPLETED_SELECT: FixedBytes<32> =
	AtomicBridgeCounterparty::BridgeTransferCompleted::SIGNATURE_HASH;
pub const COUNTERPARTY_ABORTED_SELECT: FixedBytes<32> =
	AtomicBridgeCounterparty::BridgeTransferAborted::SIGNATURE_HASH;

// Codegen for the WETH bridge contracts
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeInitiator,
	"abis/AtomicBridgeInitiator.json"
);
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeCounterparty,
	"abis/AtomicBridgeCounterparty.json"
);

// Codegen for the MOVE bridge contracts
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeInitiatorMOVE,
	"abis/AtomicBridgeInitiatorMOVE.json"
);
alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	AtomicBridgeCounterpartyMOVE,
	"abis/AtomicBridgeCounterpartyMOVE.json"
);

alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	WETH9,
	"abis/WETH9.json"
);

alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	MockMOVEToken,
	"abis/MockMOVEToken.json"
);

/// Specifies the kind of asset being transferred,
/// This will associate the client with its respective ABIs
#[derive(Debug, Clone)]
pub enum AssetKind {
	/// This will initialize the client with the WETH Bridge ABIs
	Weth,
	/// This will initialize the client with the MOVE Bridge ABIs
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

impl Default for AssetKind {
	fn default() -> Self {
		AssetKind::Move
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

#[derive(Debug, Clone)]
pub enum InitiatorContract {
	Weth(AtomicBridgeInitiator::AtomicBridgeInitiatorInstance<BoxTransport, AlloyProvider>),
	Move(AtomicBridgeInitiatorMOVE::AtomicBridgeInitiatorMOVEInstance<BoxTransport, AlloyProvider>),
}

#[derive(Debug, Clone)]
pub enum CounterpartyContract {
	Weth(AtomicBridgeCounterparty::AtomicBridgeCounterpartyInstance<BoxTransport, AlloyProvider>),
	Move(
		AtomicBridgeCounterpartyMOVE::AtomicBridgeCounterpartyMOVEInstance<
			BoxTransport,
			AlloyProvider,
		>,
	),
}

pub type WETH9Contract = WETH9::WETH9Instance<BoxTransport, AlloyProvider>;

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

// impl From<String> for EthAddress {
// 	fn from(s: String) -> Self {
// 		EthAddress(Address::parse_checksummed(s, None).expect("Invalid Ethereum address"))
// 	}
// }

impl From<Vec<u8>> for EthAddress {
	fn from(vec: Vec<u8>) -> Self {
		// Ensure the vector has the correct length
		//TODO change to a try_from but need a rewrite of
		// the address generic management to make try_from compatible.
		if vec.len() != 20 {
			tracing::warn!("Bad vec<u8> size forEthAddress conversion:{}", vec.len());
			return EthAddress(Address([0; 20].into()));
		}

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

pub(crate) enum AlloyParam {
	BridgeTransferId,
	InitiatorAddress,
	RecipientAddress,
	PreImage,
	HashLock,
	TimeLock,
	Amount,
}

impl AlloyParam {
	pub fn fill(&self) -> Param {
		match self {
			AlloyParam::BridgeTransferId => Param {
				name: "_bridgeTransferId".to_string(),
				ty: "bytes32".to_string(),
				components: vec![],
				internal_type: None,
			},
			AlloyParam::InitiatorAddress => Param {
				name: "_originator".to_string(),
				ty: "address".to_string(),
				components: vec![],
				internal_type: None,
			},
			AlloyParam::RecipientAddress => Param {
				name: "_recipient".to_string(),
				ty: "bytes32".to_string(),
				components: vec![],
				internal_type: None,
			},
			AlloyParam::PreImage => Param {
				name: "pre_image".to_string(),
				ty: "bytes32".to_string(),
				components: vec![],
				internal_type: None,
			},
			AlloyParam::HashLock => Param {
				name: "_hashLock".to_string(),
				ty: "bytes32".to_string(),
				components: vec![],
				internal_type: None,
			},
			AlloyParam::TimeLock => Param {
				name: "_timeLock".to_string(),
				ty: "uint256".to_string(),
				components: vec![],
				internal_type: None,
			},
			AlloyParam::Amount => Param {
				name: "amount".to_string(),
				ty: "uint256".to_string(),
				components: vec![],
				internal_type: None,
			},
		}
	}
}

pub(crate) enum EventName {
	InitiatorInitiated,
	InitiatorCompleted,
	InitiatorRefunded,
	CounterpartyLocked,
	CounterpartyCompleted,
	CounterpartyAborted,
}

impl EventName {
	pub fn as_str(&self) -> &str {
		match self {
			EventName::InitiatorInitiated => "BridgeTransferInitiated",
			EventName::InitiatorCompleted => "BridgeTransferCompleted",
			EventName::InitiatorRefunded => "BridgeTransferRefunded",
			EventName::CounterpartyLocked => "BridgeTransferLocked",
			EventName::CounterpartyCompleted => "BridgeTransferCompleted",
			EventName::CounterpartyAborted => "BridgeTransferAborted",
		}
	}

	pub fn params(&self) -> Vec<Param> {
		match self {
			EventName::InitiatorInitiated => vec![
				AlloyParam::BridgeTransferId.fill(),
				AlloyParam::InitiatorAddress.fill(),
				AlloyParam::RecipientAddress.fill(),
				AlloyParam::PreImage.fill(),
				AlloyParam::HashLock.fill(),
				AlloyParam::TimeLock.fill(),
				AlloyParam::Amount.fill(),
			],
			EventName::InitiatorCompleted => {
				vec![AlloyParam::BridgeTransferId.fill(), AlloyParam::PreImage.fill()]
			}
			EventName::InitiatorRefunded => vec![AlloyParam::BridgeTransferId.fill()],
			EventName::CounterpartyLocked => vec![
				AlloyParam::BridgeTransferId.fill(),
				AlloyParam::InitiatorAddress.fill(),
				AlloyParam::Amount.fill(),
				AlloyParam::HashLock.fill(),
				AlloyParam::TimeLock.fill(),
			],
			EventName::CounterpartyCompleted => {
				vec![AlloyParam::BridgeTransferId.fill(), AlloyParam::PreImage.fill()]
			}
			EventName::CounterpartyAborted => vec![AlloyParam::BridgeTransferId.fill()],
		}
	}
}

impl From<&str> for EventName {
	fn from(s: &str) -> Self {
		match s {
			"BridgeTransferInitiated" => EventName::InitiatorInitiated,
			"BridgeTransferCompleted" => EventName::InitiatorCompleted,
			"BridgeTransferRefunded" => EventName::InitiatorRefunded,
			_ => panic!("Invalid event name"),
		}
	}
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CompletedDetails<A> {
	pub bridge_transfer_id: BridgeTransferId,
	pub recipient_address: BridgeAddress<A>,
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
			recipient_address: BridgeAddress(A::from(bridge_transfer_details.recipient_address.0)),
			hash_lock: bridge_transfer_details.hash_lock,
			secret,
			amount: bridge_transfer_details.amount,
		}
	}

	pub fn from_lock_details(lock_details: LockDetails<A>, secret: HashLockPreImage) -> Self {
		CompletedDetails {
			bridge_transfer_id: lock_details.bridge_transfer_id,
			recipient_address: lock_details.recipient,
			hash_lock: lock_details.hash_lock,
			secret,
			amount: lock_details.amount,
		}
	}
}
