use std::task::{Context, Poll};

use futures::{Stream, StreamExt};

use crate::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	bridge_monitoring::{
		BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
		BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
	},
	types::{BridgeAddressType, BridgeHashType},
};

#[macro_export]
macro_rules! struct_blockchain_service {
	($Name:ident, $Address:ty, $Hash:ty, $InitiatorContract:ty, $CounterpartyContract:ty, $InitiatorMonitoring:ty, $CounterpartyMonitoring:ty) => {
		pub struct $Name {
			pub initiator_contract: $InitiatorContract,
			pub initiator_monitoring: $InitiatorMonitoring,
			pub counterparty_contract: $CounterpartyContract,
			pub counterparty_monitoring: $CounterpartyMonitoring,
		}

		impl BlockchainService for $Name {
			type Address = $Address;
			type Hash = $Hash;

			type InitiatorContract = $InitiatorContract;
			type CounterpartyContract = $CounterpartyContract;
			type InitiatorMonitoring = $InitiatorMonitoring;
			type CounterpartyMonitoring = $CounterpartyMonitoring;

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

		// NOTE For comparison in tests we only care we are the same types
		impl PartialEq for $Name {
			fn eq(&self, other: &Self) -> bool {
				use std::any::{Any, TypeId};
				TypeId::of::<Self>() == TypeId::of::<Self>()
			}
		}

		impl std::fmt::Debug for $Name {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				f.debug_struct(stringify!($Name)).finish()
			}
		}

		impl Stream for $Name {
			type Item = ContractEvent<$Address, $Hash>;

			fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
				let this = self.get_mut();
				this.poll_next_event(cx)
			}
		}
	};
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContractEvent<A, H> {
	InitiatorEvent(BridgeContractInitiatorEvent<A, H>),
	CounterpartyEvent(BridgeContractCounterpartyEvent<A, H>),
}

pub trait BlockchainService:
	Stream<Item = ContractEvent<Self::Address, Self::Hash>> + Unpin
{
	type Address: BridgeAddressType;
	type Hash: BridgeHashType;

	type InitiatorContract: BridgeContractInitiator;
	type InitiatorMonitoring: BridgeContractInitiatorMonitoring<Address = Self::Address, Hash = Self::Hash>
		+ Unpin;

	type CounterpartyContract: BridgeContractCounterparty;
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
