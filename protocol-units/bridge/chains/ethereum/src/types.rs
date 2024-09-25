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
use bridge_shared::types::{
	Amount, BridgeTransferDetails, BridgeTransferId, GenUniqueHash, HashLock, HashLockPreImage,
	InitiatorAddress, LockDetails, RecipientAddress,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

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

// Codegen from the abis
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

alloy::sol!(
	#[allow(missing_docs)]
	#[sol(rpc)]
	WETH9,
	"abis/WETH9.json"
);

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct EthHash(pub [u8; 32]);

impl EthHash {
	pub fn random() -> Self {
		let mut rng = rand::thread_rng();
		let mut hash = [0u8; 32];
		rng.fill(&mut hash);
		Self(hash)
	}
}

impl From<HashLockPreImage> for EthHash {
	fn from(value: HashLockPreImage) -> Self {
		let mut fixed_bytes = [0u8; 32];
		let len = value.0.len().min(32);
		fixed_bytes[..len].copy_from_slice(&value.0[..len]);

		Self(hash_vec_u32(&fixed_bytes))
	}
}

impl GenUniqueHash for EthHash {
	fn gen_unique_hash<R: Rng>(rng: &mut R) -> Self {
		let mut random_bytes = [0u8; 32];
		rng.fill(&mut random_bytes);
		Self(random_bytes)
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
	AtomicBridgeInitiator::AtomicBridgeInitiatorInstance<BoxTransport, AlloyProvider>;
pub type CounterpartyContract =
	AtomicBridgeCounterparty::AtomicBridgeCounterpartyInstance<BoxTransport, AlloyProvider>;
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct EthAddress(pub Address);

impl From<EthAddress> for Vec<u8> {
	fn from(address: EthAddress) -> Self {
		address.0 .0.to_vec()
	}
}

impl From<RecipientAddress<EthAddress>> for EthAddress {
	fn from(address: RecipientAddress<EthAddress>) -> Self {
		address.0
	}
}

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
pub struct CompletedDetails<A, H> {
	pub bridge_transfer_id: BridgeTransferId<H>,
	pub recipient_address: RecipientAddress<A>,
	pub hash_lock: HashLock<H>,
	pub secret: HashLockPreImage,
	pub amount: Amount,
}

impl<A, H> CompletedDetails<A, H>
where
	A: From<Vec<u8>>,
{
	pub fn from_bridge_transfer_details(
		bridge_transfer_details: BridgeTransferDetails<Vec<u8>, H>,
		secret: HashLockPreImage,
	) -> Self {
		CompletedDetails {
			bridge_transfer_id: bridge_transfer_details.bridge_transfer_id,
			recipient_address: RecipientAddress(A::from(
				bridge_transfer_details.recipient_address.0,
			)),
			hash_lock: bridge_transfer_details.hash_lock,
			secret,
			amount: bridge_transfer_details.amount,
		}
	}

	pub fn from_lock_details(lock_details: LockDetails<A, H>, secret: HashLockPreImage) -> Self {
		CompletedDetails {
			bridge_transfer_id: lock_details.bridge_transfer_id,
			recipient_address: lock_details.recipient_address,
			hash_lock: lock_details.hash_lock,
			secret,
			amount: lock_details.amount,
		}
	}
}

#[derive(Debug)]
pub enum CounterpartyCall<A, H> {
	CompleteBridgeTransfer(BridgeTransferId<H>, HashLockPreImage),
	LockBridgeTransfer(
		BridgeTransferId<H>,
		HashLock<H>,
		InitiatorAddress<Vec<u8>>,
		RecipientAddress<A>,
		Amount,
	),
}
