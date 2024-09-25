use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::types::ChainId;
use thiserror::Error;
use tokio_stream::StreamExt;

#[derive(Debug, Error)]
pub enum InvalidEventError {
	#[error("Receive an event with a bad chan id")]
	BadChain,
	#[error("Get an initiate swap event with an existing id")]
	InitAnAlreadyExist,
	#[error("Bad event received")]
	BadEvent,
	#[error("No existing state found for a non init event")]
	StateNotFound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferEvent<A> {
	pub chain: ChainId,
	pub contract_event: BridgeContractEvent<A>,
}

// #[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
// pub enum TransferEventType<AI, AR> {
// 	LockInitiatorEvent {
// 		intiator_address: InitiatorAddress<AI>,
// 		counter_part_address: RecipientAddress<AR>,
// 		hash_lock: HashLock<[u8; 32]>,
// 		time_lock: TimeLock,
// 		amount: u64,
// 	},
// 	MintLockDoneEvent,
// 	SecretEvent(Vec<u8>),
// 	MintLockFailEvent,
// 	ReleaseBurnEvent,
// 	TimeoutEvent,
// }

impl<A> From<(BridgeContractEvent<A>, ChainId)> for TransferEvent<A> {
	fn from((event, chain): (BridgeContractEvent<A>, ChainId)) -> Self {
		TransferEvent { chain, contract_event: event }
	}
}
