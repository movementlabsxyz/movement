use std::{
	pin::Pin,
	task::{Context, Poll},
};

use futures::{Stream, StreamExt};

use crate::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	bridge_monitoring::{
		BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
		BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
	},
	types::{BridgeAddressType, BridgeHashType, BridgeValueType},
};

#[derive(Debug, PartialEq, Eq)]
pub enum ContractEvent<A, H, V> {
	InitiatorEvent(BridgeContractInitiatorEvent<A, H, V>),
	CounterpartyEvent(BridgeContractCounterpartyEvent<A, H, V>),
}

pub trait BlockchainService:
	Stream<Item = ContractEvent<Self::Address, Self::Hash, Self::Value>> + Unpin
{
	type Address: BridgeAddressType;
	type Hash: BridgeHashType;
	type Value: BridgeValueType;

	type InitiatorContract: BridgeContractInitiator<Address = Self::Address, Hash = Self::Hash>;
	type InitiatorMonitoring: BridgeContractInitiatorMonitoring<Address = Self::Address, Hash = Self::Hash>
		+ Unpin;

	type CounterpartyContract: BridgeContractCounterparty<
		Address = Self::Address,
		Hash = Self::Hash,
	>;
	type CounterpartyMonitoring: BridgeContractCounterpartyMonitoring<Address = Self::Address, Hash = Self::Hash>
		+ Unpin;

	fn initiator_contract(&self) -> &Self::InitiatorContract;
	fn initiator_monitoring(&mut self) -> &mut Self::InitiatorMonitoring;
	fn counterparty_contract(&self) -> &Self::CounterpartyContract;
	fn counterparty_monitoring(&mut self) -> &mut Self::CounterpartyMonitoring;

	fn poll_next_event(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		match (
			self.initiator_monitoring().poll_next_unpin(cx),
			self.counterparty_monitoring().poll_next_unpin(cx),
		) {
			(Poll::Ready(Some(event)), _) => {
				Poll::Ready(Some(ContractEvent::InitiatorEvent(event)))
			}
			(_, Poll::Ready(Some(event))) => {
				Poll::Ready(Some(ContractEvent::CounterpartyEvent(event)))
			}
			_ => Poll::Pending,
		}
	}
}

// Practical implementation

pub struct AbstractBlockchainService<
	InitiatorContract,
	InitiatorContractMonitoring,
	CounterpartyContract,
	CounterpartyContractMonitoring,
	Address,
	Hash,
	Value,
> {
	pub initiator_contract: InitiatorContract,
	pub initiator_monitoring: InitiatorContractMonitoring,
	pub counterparty_contract: CounterpartyContract,
	pub counterparty_monitoring: CounterpartyContractMonitoring,
	pub _phantom: std::marker::PhantomData<(Address, Hash)>,
}

impl<
		InitiatorContract,
		InitiatorContractMonitoring,
		CounterpartyContract,
		CounterpartyContractMonitoring,
		Address,
		Hash,
		Value,
	> BlockchainService
	for AbstractBlockchainService<
		InitiatorContract,
		InitiatorContractMonitoring,
		CounterpartyContract,
		CounterpartyContractMonitoring,
		Address,
		Hash,
		Value,
	> where
	InitiatorContract: BridgeContractInitiator<Address = Address, Hash = Hash>,
	CounterpartyContract: BridgeContractCounterparty<Address = Address, Hash = Hash>,
	InitiatorContractMonitoring: BridgeContractInitiatorMonitoring<Address = Address, Hash = Hash>,
	CounterpartyContractMonitoring:
		BridgeContractCounterpartyMonitoring<Address = Address, Hash = Hash>,
	Address: BridgeAddressType,
	Hash: BridgeHashType,
	Value: BridgeValueType
{
	type Address = Address;
	type Hash = Hash;
	type Value = Value;

	type InitiatorContract = InitiatorContract;
	type CounterpartyContract = CounterpartyContract;
	type InitiatorMonitoring = InitiatorContractMonitoring;
	type CounterpartyMonitoring = CounterpartyContractMonitoring;

	fn initiator_contract(&self) -> &Self::InitiatorContract {
		&self.initiator_contract
	}

	fn counterparty_contract(&self) -> &Self::CounterpartyContract {
		&self.counterparty_contract
	}

	fn initiator_monitoring(&mut self) -> &mut Self::InitiatorMonitoring {
		&mut self.initiator_monitoring
	}

	fn counterparty_monitoring(&mut self) -> &mut Self::CounterpartyMonitoring {
		&mut self.counterparty_monitoring
	}
}

impl<
		InitiatorContract,
		InitiatorContractMonitoring,
		CounterpartyContract,
		CounterpartyContractMonitoring,
		Address,
		Hash,
		Value,
	> Stream
	for AbstractBlockchainService<
		InitiatorContract,
		InitiatorContractMonitoring,
		CounterpartyContract,
		CounterpartyContractMonitoring,
		Address,
		Hash,
		Value,
	> where
	InitiatorContract: BridgeContractInitiator<Address = Address, Hash = Hash>,
	CounterpartyContract: BridgeContractCounterparty<Address = Address, Hash = Hash>,
	InitiatorContractMonitoring: BridgeContractInitiatorMonitoring<Address = Address, Hash = Hash>,
	CounterpartyContractMonitoring:
		BridgeContractCounterpartyMonitoring<Address = Address, Hash = Hash>,
	Address: BridgeAddressType,
	Hash: BridgeHashType,
	Value: BridgeValueType
{
	type Item = ContractEvent<Address, Hash, Value>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		this.poll_next_event(cx)
	}
}

impl<
		InitiatorContract,
		InitiatorContractMonitoring,
		CounterpartyContract,
		CounterpartyContractMonitoring,
		Address,
		Hash,
		Value,
	> std::fmt::Debug
	for AbstractBlockchainService<
		InitiatorContract,
		InitiatorContractMonitoring,
		CounterpartyContract,
		CounterpartyContractMonitoring,
		Address,
		Hash,
		Value,
	>
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct(stringify!("AbstractBlockchainService")).finish()
	}
}
