use crate::chains::bridge_contracts::BridgeContractEvent;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InvalidEventError {
	#[error("Receive an event with a bad chan id")]
	BadChain,
	#[error("Get an initiate swap event with an existing id")]
	InitAnAlreadyExist,
	#[error("Bad event received: {0}")]
	BadEvent(String),
	#[error("No existing state found for a non init event")]
	StateNotFound,
	#[error("Error during event indexing:{0}")]
	IndexingFailed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferEvent<A> {
	pub contract_event: BridgeContractEvent<A>,
}

impl<A> From<BridgeContractEvent<A>> for TransferEvent<A> {
	fn from(event: BridgeContractEvent<A>) -> Self {
		TransferEvent { contract_event: event }
	}
}

impl<A: std::fmt::Debug> fmt::Display for TransferEvent<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"Event: {} => {:?}",
			self.contract_event.bridge_transfer_id(),
			self.contract_event,
		)
	}
}
