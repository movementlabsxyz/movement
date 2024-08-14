#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovementChainEvent<A, H> {
	InitiatorContractEvent(SCIResult<A, H>),
	CounterpartyContractEvent(SCCResult<A, H>),
	Noop,
}
