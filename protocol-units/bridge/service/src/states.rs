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

impl TransferState {
	pub fn validate_event<A>(&self, event: &TransferEvent<A>) -> Result<(), InvalidEventError> {
		match (&event.contract_event, &self.state) {
			(BridgeContractEvent::Initiated(_), _) => {
				// already present invalid
				Err(InvalidEventError::InitAnAlreadyExist)
			}
			// Lock event must on on the counter part chain.
			(BridgeContractEvent::Locked(_), TransferStateType::Initialized) => (event.chain
				!= self.init_chain)
				.then_some(())
				.ok_or(InvalidEventError::BadChain),
			// Mint event is only applied on Initialized swap state
			(BridgeContractEvent::Locked(_), _) => Err(InvalidEventError::BadEvent),
			//TODO
			(BridgeContractEvent::InitialtorCompleted(_), _) => todo!(),
			(BridgeContractEvent::CounterPartCompleted(_, _), _) => todo!(),
			(BridgeContractEvent::Refunded(_), _) => todo!(),
			(&BridgeContractEvent::Cancelled(_), _) => todo!(),
		}
	}

	pub fn transition_from_initiated<A: Into<Vec<u8>> + Clone>(
		chain_id: ChainId,
		transfer_id: BridgeTransferId,
		detail: BridgeTransferDetails<A>,
	) -> (Self, TransferAction) {
		let state = TransferState {
			state: TransferStateType::Initialized,
			init_chain: chain_id,
			transfer_id,
			intiator_address: detail.initiator_address.clone().into(),
			counter_part_address: detail.recipient_address.clone().into(),
			hash_lock: detail.hash_lock,
			time_lock: detail.time_lock,
			amount: detail.amount,
			contract_state: detail.state,
			retry_on_error: 0,
		};

		let action_type = TransferActionType::LockBridgeTransfer {
			bridge_transfer_id: transfer_id,
			hash_lock: detail.hash_lock,
			initiator: BridgeAddress(detail.initiator_address.0.into()),
			recipient: BridgeAddress(detail.recipient_address.0.into()),
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

	pub fn transition_to_refund(&self) -> (TransferStateType, TransferActionType) {
		(TransferStateType::Refund, TransferActionType::RefundInitiator)
	}
}
