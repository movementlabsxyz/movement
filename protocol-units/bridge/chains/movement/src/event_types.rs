use bridge_shared::{counterparty_contract::SCCResult, initiator_contract::SCIResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovementChainEvent<A, H> {
	InitiatorContractEvent(SCIResult<A, H>),
	CounterpartyContractEvent(SCCResult<A, H>),
	Noop,
}

pub enum CounterpartyEventKind {
	Locked,
	Completed,
}
