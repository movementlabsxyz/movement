use futures::Stream;

use crate::types::{
	BridgeTransferDetails, BridgeTransferId, CounterpartyCompletedDetails, LockDetails,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeContractInitiatorEvent<A, H, V> {
	Initiated(BridgeTransferDetails<A, H, V>),
	Completed(BridgeTransferId<H>),
	Refunded(BridgeTransferId<H>),
}

impl<A, H, V> BridgeContractInitiatorEvent<A, H, V> {
	pub fn bridge_transfer_id(&self) -> &BridgeTransferId<H> {
		match self {
			Self::Initiated(details) => &details.bridge_transfer_id,
			Self::Completed(id) | Self::Refunded(id) => id,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeContractCounterpartyEvent<A, H, V> {
	Locked(LockDetails<A, H, V>),
	Completed(CounterpartyCompletedDetails<A, H, V>),
}

pub trait BridgeContractInitiatorMonitoring:
	Stream<Item = BridgeContractInitiatorEvent<Self::Address, Self::Hash, Self::Value>> + Unpin
{
	type Address;
	type Hash;
	type Value;
}

pub trait BridgeContractCounterpartyMonitoring:
	Stream<Item = BridgeContractCounterpartyEvent<Self::Address, Self::Hash, Self::Value>> + Unpin
{
	type Address;
	type Hash;
	type Value;
}
