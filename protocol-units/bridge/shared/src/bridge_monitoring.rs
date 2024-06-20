use futures::Stream;

use crate::types::{BridgeTransferDetails, BridgeTransferId, LockDetails, UnlockDetails};

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeContractInitiatorEvent<A, H> {
	Initiated(BridgeTransferDetails<A, H>),
	Completed(BridgeTransferId<H>),
	Refunded(BridgeTransferId<H>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeContractCounterpartyEvent<A, H> {
	Locked(LockDetails<A, H>),
	Unlocked(UnlockDetails<A, H>),
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
