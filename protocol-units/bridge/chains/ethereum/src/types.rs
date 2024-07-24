use alloy::json_abi::Param;
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterpartyError, BridgeContractInitiatorError},
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
};

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

#[derive(Debug, Clone, Copy, Default, Hash, Eq, PartialEq)]
pub(crate) struct EthHash(pub(crate) [u8; 32]);

impl From<Vec<u8>> for EthHash {
	fn from(vec: Vec<u8>) -> Self {
		let mut array = [0u8; 32];
		let bytes = &vec[..std::cmp::min(vec.len(), 32)];
		array[..bytes.len()].copy_from_slice(bytes);
		EthHash(array)
	}
}

impl EthHash {
	pub fn as_bytes(&self) -> [u8; 32] {
		self.0
	}
}

pub(crate) type SCIResult<A, H> =
	Result<BridgeContractInitiatorEvent<A, H>, BridgeContractInitiatorError>;
pub(crate) type SCCResult<H> =
	Result<BridgeContractCounterpartyEvent<H>, BridgeContractCounterpartyError>;

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
