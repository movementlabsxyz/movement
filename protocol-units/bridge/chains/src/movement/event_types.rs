use bridge_shared::counterparty_contract::SCCResult;
use bridge_shared::initiator_contract::SCIResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovementChainEvent<A, H> {
	InitiatorContractEvent(SCIResult<A, H>),
	CounterpartyContractEvent(SCCResult<A, H>),
	Noop,
}
