use futures::Stream;

use crate::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	bridge_monitoring::{
		BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
		BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
	},
};

#[derive(Debug, PartialEq, Eq)]
pub enum BlockchainEvent<A, H> {
	InitiatorEvent(BridgeContractInitiatorEvent<A, H>),
	CounterpartyEvent(BridgeContractCounterpartyEvent<A, H>),
}

pub trait BlockchainService:
	Stream<Item = BlockchainEvent<Self::Address, Self::Hash>> + Unpin
{
	type Address;
	type Hash;

	type InitiatorContract: BridgeContractInitiator<Self::Address, Self::Hash>;
	type InitiatorMonitoring: BridgeContractInitiatorMonitoring<Self::Address, Self::Hash>;

	type CounterpartyContract: BridgeContractCounterparty<Self::Address, Self::Hash>;
	type CounterpartyMonitoring: BridgeContractCounterpartyMonitoring<Self::Address, Self::Hash>;

	fn initiator_contract(&self) -> &Self::InitiatorContract;
	fn initiator_monitoring(&self) -> &Self::InitiatorMonitoring;
	fn counterparty_contract(&self) -> &Self::CounterpartyContract;
	fn counterparty_monitoring(&self) -> &Self::CounterpartyMonitoring;
}
