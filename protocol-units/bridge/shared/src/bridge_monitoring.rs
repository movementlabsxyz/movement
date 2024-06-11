use futures::Stream;

use crate::types::{BridgeTransferDetails, BridgeTransferId};

pub enum BridgeContractInitiatorEvent<A, H> {
	BridgeTransferInitiated(BridgeTransferDetails<A, H>),
	BridgeTransferCompleted(BridgeTransferId<H>),
	BridgeTransferRefunded(BridgeTransferId<H>),
}

pub enum BridgeContractCounterpartyEvent<A, H> {
	BridgeTransferLocked(BridgeTransferDetails<A, H>),
}

pub trait BridgeContractInitiatorMonitoring<A, H>:
	Stream<Item = BridgeContractInitiatorEvent<A, H>>
{
}

pub trait BridgeContractCounterpartyMonitoring<A, H>:
	Stream<Item = BridgeContractCounterpartyEvent<A, H>>
{
}
