use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::events::{InvalidEventError, TransferEvent};
use crate::types::Amount;
use crate::types::BridgeAddress;
use crate::types::BridgeTransferDetails;
use crate::types::HashLockPreImage;
use crate::types::LockDetails;
use crate::types::{BridgeTransferId, ChainId, HashLock, TimeLock};
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
	Locked,
	SecretReceived,
	CompletedIntiator,
	Done,
	Refund,
}

impl fmt::Display for TransferStateType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let kind = match self {
			Self::Initialized => "Initialized",
			Self::Locked => "Locked",
			Self::SecretReceived => "SecretReceived",
			Self::CompletedIntiator => "CompletedIntiator",
			Self::Done => "Done",
			Self::Refund => "Refund",
		};
		write!(f, "{}", kind,)
	}
}

#[allow(dead_code)]
pub struct TransferState {
	pub state: TransferStateType,
	pub init_chain: ChainId,
	pub transfer_id: BridgeTransferId,
	pub intiator_address: TransferAddress,
	pub counter_part_address: TransferAddress,
	pub hash_lock: HashLock,
	pub time_lock: TimeLock,
	pub amount: Amount,
	pub contract_state: u8,
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
			// Lock event must be on the counter part chain.
			(BridgeContractEvent::Locked(_), TransferStateType::Initialized) => (event.chain
				!= self.init_chain)
				.then_some(())
				.ok_or(InvalidEventError::BadChain),
			// Lock event is only applied on Initialized swap state
			(BridgeContractEvent::Locked(details), _) => Err(InvalidEventError::BadEvent(format!("Received a locked event with state not Initialized, transfer_id: {} state:{} details: {details:?}", self.transfer_id, self.state))),
			// CounterPartCompleted event must be on the counter part chain.
			(BridgeContractEvent::CounterPartyCompleted(_, _), TransferStateType::Locked) => {
				(event.chain != self.init_chain)
					.then_some(())
					.ok_or(InvalidEventError::BadChain)
			}
			// CounterPartCompleted event is only applied on Locked swap state
			(BridgeContractEvent::CounterPartyCompleted(_, _), _) => {
				Err(InvalidEventError::BadEvent(format!("Received a CounterPartCompleted event with state not Locked, transfer_id: {} state:{}", self.transfer_id, self.state)))
			}
			// InitiatorCompleted event must be on the init chain.
			(BridgeContractEvent::InitiatorCompleted(_), TransferStateType::SecretReceived) => {
				(event.chain == self.init_chain)
					.then_some(())
					.ok_or(InvalidEventError::BadChain)
			}
			(BridgeContractEvent::InitiatorCompleted(_), _) => Err(InvalidEventError::BadEvent(format!("Received a InitialtorCompleted event with state not SecretReceived, transfer_id: {} state:{}", self.transfer_id, self.state))),
			(BridgeContractEvent::Refunded(_), _) => Ok(()),
			(&BridgeContractEvent::Cancelled(_), _) => Ok(()),
		}
	}

	pub fn transition_from_initiated<A: Into<Vec<u8>> + Clone>(
		chain_id: ChainId,
		transfer_id: BridgeTransferId,
		detail: BridgeTransferDetails<A>,
	) -> (Self, TransferAction) {
		println!("State transition_from_initiated amount {:?}", detail.amount);

		let state = TransferState {
			state: TransferStateType::Initialized,
			init_chain: chain_id,
			transfer_id,
			intiator_address: detail.initiator.clone().into(),
			counter_part_address: detail.recipient.clone().into(),
			hash_lock: detail.hash_lock,
			time_lock: detail.time_lock,
			amount: detail.amount,
			contract_state: detail.state,
			retry_on_error: 0,
		};

		let action_type = TransferActionType::LockBridgeTransfer {
			bridge_transfer_id: transfer_id,
			hash_lock: detail.hash_lock,
			initiator: BridgeAddress(detail.initiator.0.into()),
			recipient: BridgeAddress(detail.recipient.0.into()),
			amount: detail.amount,
		};
		let action = TransferAction { chain: chain_id, transfer_id, kind: action_type };
		(state, action)
	}

	pub fn transition_from_locked_done<A: Into<Vec<u8>> + Clone>(
		mut self,
		_transfer_id: BridgeTransferId,
		_detail: LockDetails<A>,
	) -> (Self, TransferActionType) {
		self.state = TransferStateType::Locked;
		let action_type = TransferActionType::NoAction;
		(self, action_type)
	}

	pub fn transition_from_counterpart_completed(
		mut self,
		_transfer_id: BridgeTransferId,
		secret: HashLockPreImage,
	) -> (Self, TransferActionType) {
		self.state = TransferStateType::SecretReceived;
		let action_type = TransferActionType::WaitAndCompleteInitiator(0, secret);
		(self, action_type)
	}

	pub fn transition_from_initiator_completed(
		mut self,
		_transfer_id: BridgeTransferId,
	) -> (Self, TransferActionType) {
		self.state = TransferStateType::Done;
		let action_type = TransferActionType::NoAction;
		(self, action_type)
	}

	pub fn transition_from_cancelled(
		mut self,
		_transfer_id: BridgeTransferId,
	) -> (Self, TransferActionType) {
		self.state = TransferStateType::Done;
		let action_type = TransferActionType::NoAction;
		(self, action_type)
	}

	pub fn transition_from_refunded(
		mut self,
		_transfer_id: BridgeTransferId,
	) -> (Self, TransferActionType) {
		self.state = TransferStateType::Done;
		let action_type = TransferActionType::NoAction;
		(self, action_type)
	}

	pub fn transition_to_refund(&self) -> (TransferStateType, TransferActionType) {
		(TransferStateType::Refund, TransferActionType::RefundInitiator)
	}
}
