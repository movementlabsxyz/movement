use futures::Stream;

use crate::types::{BridgeTransferDetails, BridgeTransferId};

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeContractInitiatorEvent<A, H> {
	BridgeTransferInitiated(BridgeTransferDetails<A, H>),
	BridgeTransferCompleted(BridgeTransferId<H>),
	BridgeTransferRefunded(BridgeTransferId<H>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeContractCounterpartyEvent<A, H> {
	BridgeTransferLocked(BridgeTransferDetails<A, H>),
}

pub trait BridgeContractInitiatorMonitoring:
	Stream<Item = BridgeContractInitiatorEvent<Self::Address, Self::Hash>>
{
	type Address;
	type Hash;
}

pub trait BridgeContractCounterpartyMonitoring:
	Stream<Item = BridgeContractCounterpartyEvent<Self::Address, Self::Hash>>
{
	type Address;
	type Hash;
}
