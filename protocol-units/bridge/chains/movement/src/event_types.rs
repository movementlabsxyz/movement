use bridge_shared::types::SCIResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovementChainEvent<A, H> {
	InitiatorContractEvent(SCIResult<A, H>),
	CounterpartyContractEvent(SCCResult<A, H>),
	Noop,
}

pub enum InitiatorEventKind {
	Initiated,
	Completed,
	Refunded,
}

pub enum CounterpartyEventKind {
	Locked,
	Completed,
	Cancelled,
}
