use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::events::{InvalidEventError, TransferEvent};
use crate::types::Amount;
use crate::types::BridgeAddress;
use crate::types::{BridgeTransferId, ChainId, HashLock, TimeLock};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransferAddress(Vec<u8>);

impl<A: Into<Vec<u8>>> From<BridgeAddress<A>> for TransferAddress {
	fn from(addr: BridgeAddress<A>) -> Self {
		TransferAddress(addr.0.into())
	}
}

impl<A: From<Vec<u8>>> From<TransferAddress> for BridgeAddress<A> {
	fn from(addr: TransferAddress) -> Self {
		BridgeAddress(Vec::<u8>::from(addr.0))
	}
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum TransferStateType {
	Initialized,
	Locked,
	CompetedIntiator,
	CompetedCounterPart,
	Done,
	NeedRefund,
}

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
			(BridgeContractEvent::CounterPartCompleted(_), _) => todo!(),
			(BridgeContractEvent::Refunded(_), _) => todo!(),
		}
	}
}
