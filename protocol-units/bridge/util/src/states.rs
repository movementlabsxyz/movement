use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::events::{InvalidEventError, TransferEvent};
use crate::types::Amount;
use crate::types::BridgeAddress;
use crate::types::{BridgeTransferId, ChainId};
use crate::types::{BridgeTransferInitiatedDetails, Nonce};
use crate::TransferAction;
use crate::TransferActionType;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferAddress(Vec<u8>);

impl<A: Into<Vec<u8>>> From<BridgeAddress<A>> for TransferAddress {
	fn from(addr: BridgeAddress<A>) -> Self {
		TransferAddress(addr.0.into())
	}
}

impl<A: From<Vec<u8>>> From<TransferAddress> for BridgeAddress<A> {
	fn from(addr: TransferAddress) -> Self {
		BridgeAddress(addr.0.into())
	}
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum TransferStateType {
	Initialized,
	Completed,
}

impl fmt::Display for TransferStateType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let kind = match self {
			Self::Initialized => "Initialized",
			Self::Completed => "Completed",
		};
		write!(f, "{}", kind,)
	}
}

#[allow(dead_code)]
pub struct TransferState {
	pub state: TransferStateType,
	pub init_chain: ChainId,
	pub transfer_id: BridgeTransferId,
	pub initiator: TransferAddress,
	pub recipient: TransferAddress,
	pub amount: Amount,
	pub nonce: Nonce,
	//Max number time action are retry for the whole transfer.
	pub retry_on_error: usize,
}

impl fmt::Display for TransferState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"Transfer State: {} / transfer id: {} / init_chain: {} ",
			self.state, self.transfer_id, self.init_chain
		)
	}
}

impl TransferState {
	pub fn validate_event<A: std::fmt::Debug>(
		&self,
		event: &TransferEvent<A>,
	) -> Result<(), InvalidEventError> {
		match (&event.contract_event, &self.state) {
			(BridgeContractEvent::Initiated(_), _) => {
				// already present invalid
				Err(InvalidEventError::InitAnAlreadyExist)
			}
			// Complete event must on on the counter part chain.
			(BridgeContractEvent::Completed(_), TransferStateType::Initialized) => (event.chain
				!= self.init_chain)
				.then_some(())
				.ok_or(InvalidEventError::BadChain),

			_ => Err(InvalidEventError::BadEvent(format!(
				"Received an invalid event {} with state not Initialized, transfer_id: {} state:{}",
				event.contract_event, self.transfer_id, self.state
			))),
		}
	}

	pub fn transition_from_initiated<A: Into<Vec<u8>> + Clone>(
		chain_id: ChainId,
		transfer_id: BridgeTransferId,
		detail: BridgeTransferInitiatedDetails<A>,
	) -> (Self, TransferAction) {
		println!("State transition_from_initiated amount {:?}", detail.amount);

		let state = TransferState {
			state: TransferStateType::Initialized,
			init_chain: chain_id,
			transfer_id,
			initiator: detail.initiator.clone().into(),
			recipient: detail.recipient.clone().into(),
			amount: detail.amount,
			nonce: detail.nonce,
			retry_on_error: 0,
		};

		let action_type = TransferActionType::CompleteBridgeTransfer {
			bridge_transfer_id: transfer_id,
			initiator: BridgeAddress(detail.initiator.0.into()),
			recipient: BridgeAddress(detail.recipient.0.into()),
			amount: detail.amount,
			nonce: detail.nonce,
		};
		let action = TransferAction { chain: chain_id, transfer_id, kind: action_type };
		(state, action)
	}

	pub fn transition_from_completed(
		mut self,
		_transfer_id: BridgeTransferId,
	) -> (Self, TransferActionType) {
		self.state = TransferStateType::Completed;
		let action_type = TransferActionType::CompletedRemoveState;
		(self, action_type)
	}

	pub fn transition_from_aborted(&mut self, transfer_id: BridgeTransferId) -> TransferActionType {
		self.state = TransferStateType::Initialized;
		let action_type = TransferActionType::AbortedReplay {
			bridge_transfer_id: transfer_id,
			initiator: BridgeAddress(self.initiator.0.clone().into()),
			recipient: BridgeAddress(self.recipient.0.clone().into()),
			amount: self.amount,
			nonce: self.nonce,
			wait_time_sec: 10, //TODO set wait time in config.
		};

		action_type
	}
}
