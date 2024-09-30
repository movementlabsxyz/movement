use crate::chains::bridge_contracts::BridgeContractEvent;
use crate::types::ChainId;
use std::fmt;
use thiserror::Error;

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

impl<A: std::fmt::Debug> fmt::Display for TransferEvent<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"Event: {}: {} => {:?}",
			self.chain,
			self.contract_event.bridge_transfer_id(),
			self.contract_event,
		)
	}
}

// impl<A>
// 	From<(
// 		BridgeContractEvent<
// 			<dyn BridgeContractMonitoring<
// 				Address = A,
// 				Item = BridgeContractResult<BridgeContractEvent<A>>,
// 			> as BridgeContractMonitoring>::Address,
// 		>,
// 		ChainId,
// 	)>
// 	for TransferEvent<
// 		<dyn BridgeContractMonitoring<
// 			Address = A,
// 			Item = BridgeContractResult<BridgeContractEvent<A>>,
// 		> as BridgeContractMonitoring>::Address,
// 	>
// {
// 	fn from(
// 		(event, chain): (
// 			BridgeContractEvent<
// 				<dyn BridgeContractMonitoring<
// 					Address = A,
// 					Item = BridgeContractResult<BridgeContractEvent<A>>,
// 				> as BridgeContractMonitoring>::Address,
// 			>,
// 			ChainId,
// 		),
// 	) -> Self {
// 		TransferEvent { chain, contract_event: event }
// 	}
// }
