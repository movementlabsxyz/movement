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

impl<A> From<(BridgeContractEvent<A>, ChainId)> for TransferEvent<A> {
	fn from((event, chain): (BridgeContractEvent<A>, ChainId)) -> Self {
		TransferEvent { chain, contract_event: event }
	}
}
