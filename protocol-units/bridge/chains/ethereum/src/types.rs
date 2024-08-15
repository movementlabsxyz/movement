use std::collections::HashMap;

use alloy::json_abi::Param;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::primitives::{Address, FixedBytes};
use alloy::providers::fillers::{
	ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::RootProvider;
use alloy::pubsub::PubSubFrontend;
use alloy::rlp::{RlpDecodable, RlpEncodable};
use alloy::sol_types::SolEvent;
use alloy::transports::BoxTransport;
use bridge_shared::types::{
	Amount, BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId,
	GenUniqueHash, HashLock, HashLockPreImage, InitiatorAddress, LockDetails, RecipientAddress,
	TimeLock,
};
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterpartyError, BridgeContractInitiatorError},
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
};
use futures::channel::mpsc::UnboundedReceiver;
use serde::{Deserialize, Serialize};

use crate::AtomicBridgeInitiator::AtomicBridgeInitiatorInstance;
use crate::AtomicBridgeCounterparty::AtomicBridgeCounterpartyInstance;
use crate::WETH9::WETH9Instance;
pub const INITIATED_SELECT: FixedBytes<32> =
	AtomicBridgeInitiator::BridgeTransferInitiated::SIGNATURE_HASH;
pub const COMPLETED_SELECT: FixedBytes<32> =
	AtomicBridgeInitiator::BridgeTransferCompleted::SIGNATURE_HASH;
pub const REFUNDED_SELECT: FixedBytes<32> =
	AtomicBridgeInitiator::BridgeTransferRefunded::SIGNATURE_HASH;

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

pub type EthHash = [u8; 32];

pub type InitiatorContract = AtomicBridgeInitiatorInstance<BoxTransport, AlloyProvider>;
pub type CounterpartyContract = AtomicBridgeCounterpartyInstance<BoxTransport, AlloyProvider>;
pub type WETH9Contract = WETH9Instance<BoxTransport, AlloyProvider>;

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
	Initiated,
	Completed,
	Refunded,
}

impl EventName {
	pub fn as_str(&self) -> &str {
		match self {
			EventName::Initiated => "BridgeTransferInitiated",
			EventName::Completed => "BridgeTransferCompleted",
			EventName::Refunded => "BridgeTransferRefunded",
		}
	}

	pub fn params(&self) -> Vec<Param> {
		match self {
			EventName::Initiated => vec![
				AlloyParam::BridgeTransferId.fill(),
				AlloyParam::InitiatorAddress.fill(),
				AlloyParam::RecipientAddress.fill(),
				AlloyParam::PreImage.fill(),
				AlloyParam::HashLock.fill(),
				AlloyParam::TimeLock.fill(),
				AlloyParam::Amount.fill(),
			],
			EventName::Completed => {
				vec![AlloyParam::BridgeTransferId.fill(), AlloyParam::PreImage.fill()]
			}
			EventName::Refunded => vec![AlloyParam::BridgeTransferId.fill()],
		}
	}
}

impl From<&str> for EventName {
	fn from(s: &str) -> Self {
		match s {
			"BridgeTransferInitiated" => EventName::Initiated,
			"BridgeTransferCompleted" => EventName::Completed,
			"BridgeTransferRefunded" => EventName::Refunded,
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
		TimeLock,
		InitiatorAddress<Vec<u8>>,
		RecipientAddress<A>,
		Amount,
	),
}
