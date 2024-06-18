use futures::Stream;

use crate::types::{BridgeTransferDetails, BridgeTransferId, LockedAssetsDetails};

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeContractInitiatorEvent<A, H> {
	BridgeTransferInitiated(BridgeTransferDetails<A, H>),
	BridgeTransferCompleted(BridgeTransferId<H>),
	BridgeTransferRefunded(BridgeTransferId<H>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeContractCounterpartyEvent<A, H> {
	BridgeTransferLocked(LockedAssetsDetails<A, H>),
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
