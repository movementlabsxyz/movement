use crate::chains::bridge_contracts::BridgeContractError;
use crate::types::{Amount, BridgeAddress, BridgeTransferId, Nonce};
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
	pub transfer_id: BridgeTransferId,
	pub kind: TransferActionType,
}
impl fmt::Display for TransferAction {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Action: {} / {}", self.transfer_id, self.kind)
	}
}

#[derive(Debug, Clone)]
pub enum TransferActionType {
	CompleteBridgeTransfer {
		bridge_transfer_id: BridgeTransferId,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<Vec<u8>>,
		amount: Amount,
		nonce: Nonce,
	},
	CompletedRemoveState,
	AbortedReplay {
		bridge_transfer_id: BridgeTransferId,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<Vec<u8>>,
		amount: Amount,
		nonce: Nonce,
		wait_time_sec: u64,
	},
	NoAction,
}

impl fmt::Display for TransferActionType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let act = match self {
			TransferActionType::CompleteBridgeTransfer { .. } => "CompleteBridgeTransfer",
			TransferActionType::CompletedRemoveState => "CompletedRemoveState",
			TransferActionType::AbortedReplay { .. } => "AbortedReplay",
			TransferActionType::NoAction => "NoAction",
		};
		write!(f, "{}", act)
	}
}
