use crate::chains::bridge_contracts::BridgeContractError;
use crate::types::ChainId;
use crate::types::{Amount, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub struct ActionExecError(pub TransferAction, pub BridgeContractError);

impl ActionExecError {
	pub fn inner(self) -> (TransferAction, BridgeContractError) {
		(self.0, self.1)
	}
}

impl fmt::Display for ActionExecError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Action: {}/ Error: {}", self.0, self.1,)
	}
}

#[derive(Debug, Clone)]
pub struct TransferAction {
	pub chain: ChainId,
	pub transfer_id: BridgeTransferId,
	pub kind: TransferActionType,
}
impl fmt::Display for TransferAction {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Action: {}/{}/{}", self.chain, self.transfer_id, self.kind)
	}
}

#[derive(Debug, Clone)]
pub enum TransferActionType {
	LockBridgeTransfer {
		bridge_transfer_id: BridgeTransferId,
		hash_lock: HashLock,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<Vec<u8>>,
		amount: Amount,
	},
	WaitAndCompleteInitiator(u64, HashLockPreImage),
	RefundInitiator,
	TransferDone,
	NoAction,
}

impl fmt::Display for TransferActionType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let act = match self {
			TransferActionType::LockBridgeTransfer { .. } => "LockBridgeTransfer",
			TransferActionType::WaitAndCompleteInitiator(..) => "WaitAndCompleteInitiator",
			TransferActionType::RefundInitiator => "RefundInitiator",
			TransferActionType::TransferDone => "TransferDone",
			TransferActionType::NoAction => "NoAction",
		};
		write!(f, "{}", act)
	}
}
