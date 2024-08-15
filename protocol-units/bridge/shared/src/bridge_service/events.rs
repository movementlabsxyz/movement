use crate::{
	blockchain_service::BlockchainService,
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
	types::{BridgeTransferDetails, BridgeTransferId, CounterpartyCompletedDetails},
};

use super::active_swap::LockBridgeTransferAssetsError;

#[derive(Debug, PartialEq, Eq)]
pub enum IWarn<A, H, V> {
	AlreadyPresent(BridgeTransferDetails<A, H, V>),
	CompleteTransferError(BridgeTransferId<H>),
	CompletionAbortedTooManyAttempts(BridgeTransferId<H>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum IEvent<A, H, V> {
	ContractEvent(BridgeContractInitiatorEvent<A, H, V>),
	Warn(IWarn<A, H, V>),
	RetryCompletingTransfer(BridgeTransferId<H>),
}

impl<A, H, V> IEvent<A, H, V> {
	pub fn contract_event(&self) -> Option<&BridgeContractInitiatorEvent<A, H, V>> {
		match self {
			IEvent::ContractEvent(event) => Some(event),
			_ => None,
		}
	}
	pub fn warn(&self) -> Option<&IWarn<A, H, V>> {
		match self {
			IEvent::Warn(warn) => Some(warn),
			_ => None,
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub enum CWarn<A, H, V> {
	BridgeAssetsLockingError(LockBridgeTransferAssetsError),
	CannotCompleteUnexistingSwap(CounterpartyCompletedDetails<A, H, V>),
	LockingAbortedTooManyAttempts(BridgeTransferId<H>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum CEvent<A, H, V> {
	RetryLockingAssets(BridgeTransferId<H>),
	ContractEvent(BridgeContractCounterpartyEvent<A, H, V>),
	Warn(CWarn<A, H, V>),
}

impl<A, H, V> CEvent<A, H, V> {
	pub fn contract_event(&self) -> Option<&BridgeContractCounterpartyEvent<A, H, V>> {
		match self {
			CEvent::ContractEvent(event) => Some(event),
			_ => None,
		}
	}

	pub fn warn(&self) -> Option<&CWarn<A, H, V>> {
		match self {
			CEvent::Warn(warn) => Some(warn),
			_ => None,
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event<B1, B2, V>
where
	B1: BlockchainService,
	B2: BlockchainService,
{
	B1I(IEvent<B1::Address, B1::Hash, V>),
	B1C(CEvent<B1::Address, B1::Hash, V>),
	B2I(IEvent<B2::Address, B2::Hash, V>),
	B2C(CEvent<B2::Address, B2::Hash, V>),
}

#[allow(non_snake_case)]
impl<B1: BlockchainService, B2: BlockchainService, V> Event<B1, B2, V> {
	pub fn B1I(&self) -> Option<&IEvent<B1::Address, B1::Hash, V>> {
		match self {
			Event::B1I(event) => Some(event),
			_ => None,
		}
	}
	pub fn B1I_ContractEvent(
		&self,
	) -> Option<&BridgeContractInitiatorEvent<B1::Address, B1::Hash, V>> {
		self.B1I()?.contract_event()
	}

	pub fn B1C(&self) -> Option<&CEvent<B1::Address, B1::Hash, V>> {
		match self {
			Event::B1C(event) => Some(event),
			_ => None,
		}
	}

	pub fn B1C_ContractEvent(
		&self,
	) -> Option<&BridgeContractCounterpartyEvent<B1::Address, B1::Hash, V>> {
		self.B1C()?.contract_event()
	}

	pub fn B2I(&self) -> Option<&IEvent<B2::Address, B2::Hash, V>> {
		match self {
			Event::B2I(event) => Some(event),
			_ => None,
		}
	}
	pub fn B2I_ContractEvent(
		&self,
	) -> Option<&BridgeContractInitiatorEvent<B2::Address, B2::Hash, V>> {
		self.B2I()?.contract_event()
	}

	pub fn B2C(&self) -> Option<&CEvent<B2::Address, B2::Hash, V>> {
		match self {
			Event::B2C(event) => Some(event),
			_ => None,
		}
	}

	pub fn B2C_ContractEvent(
		&self,
	) -> Option<&BridgeContractCounterpartyEvent<B2::Address, B2::Hash, V>> {
		self.B2C()?.contract_event()
	}
}
