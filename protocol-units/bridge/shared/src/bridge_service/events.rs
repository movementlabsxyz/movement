use crate::{
	blockchain_service::BlockchainService,
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
	types::{BridgeTransferDetails, BridgeTransferId, CompletedDetails},
};

use super::active_swap::LockBridgeTransferAssetsError;

#[derive(Debug, PartialEq, Eq)]
pub enum IWarn<A, H> {
	AlreadyPresent(BridgeTransferDetails<A, H>),
	CompleteTransferError(BridgeTransferId<H>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum IEvent<A, H> {
	ContractEvent(BridgeContractInitiatorEvent<A, H>),
	Warn(IWarn<A, H>),
	RetryCompletingTransfer(BridgeTransferId<H>),
}

impl<A, H> IEvent<A, H> {
	pub fn contract_event(&self) -> Option<&BridgeContractInitiatorEvent<A, H>> {
		match self {
			IEvent::ContractEvent(event) => Some(event),
			_ => None,
		}
	}
	pub fn warn(&self) -> Option<&IWarn<A, H>> {
		match self {
			IEvent::Warn(warn) => Some(warn),
			_ => None,
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub enum CWarn<H> {
	BridgeAssetsLockingError(LockBridgeTransferAssetsError),
	CannotCompleteUnexistingSwap(CompletedDetails<H>),
	AbortedTooManyAttempts(BridgeTransferId<H>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum CEvent<H> {
	RetryLockingAssets(BridgeTransferId<H>),
	ContractEvent(BridgeContractCounterpartyEvent<H>),
	Warn(CWarn<H>),
}

impl<H> CEvent<H> {
	pub fn contract_event(&self) -> Option<&BridgeContractCounterpartyEvent<H>> {
		match self {
			CEvent::ContractEvent(event) => Some(event),
			_ => None,
		}
	}

	pub fn warn(&self) -> Option<&CWarn<H>> {
		match self {
			CEvent::Warn(warn) => Some(warn),
			_ => None,
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event<B1, B2>
where
	B1: BlockchainService,
	B2: BlockchainService,
{
	B1I(IEvent<B1::Address, B1::Hash>),
	B1C(CEvent<B1::Hash>),
	B2I(IEvent<B2::Address, B2::Hash>),
	B2C(CEvent<B2::Hash>),
}

#[allow(non_snake_case)]
impl<B1: BlockchainService, B2: BlockchainService> Event<B1, B2> {
	pub fn B1I(&self) -> Option<&IEvent<B1::Address, B1::Hash>> {
		match self {
			Event::B1I(event) => Some(event),
			_ => None,
		}
	}
	pub fn B1I_ContractEvent(
		&self,
	) -> Option<&BridgeContractInitiatorEvent<B1::Address, B1::Hash>> {
		self.B1I()?.contract_event()
	}

	pub fn B1C(&self) -> Option<&CEvent<B1::Hash>> {
		match self {
			Event::B1C(event) => Some(event),
			_ => None,
		}
	}

	pub fn B1C_ContractEvent(&self) -> Option<&BridgeContractCounterpartyEvent<B1::Hash>> {
		self.B1C()?.contract_event()
	}

	pub fn B2I(&self) -> Option<&IEvent<B2::Address, B2::Hash>> {
		match self {
			Event::B2I(event) => Some(event),
			_ => None,
		}
	}
	pub fn B2I_ContractEvent(
		&self,
	) -> Option<&BridgeContractInitiatorEvent<B2::Address, B2::Hash>> {
		self.B2I()?.contract_event()
	}

	pub fn B2C(&self) -> Option<&CEvent<B2::Hash>> {
		match self {
			Event::B2C(event) => Some(event),
			_ => None,
		}
	}

	pub fn B2C_ContractEvent(&self) -> Option<&BridgeContractCounterpartyEvent<B2::Hash>> {
		self.B2C()?.contract_event()
	}
}
