use bridge_shared::{
	bridge_monitoring::BridgeContractInitiatorEvent,
	counterparty_contract::SCCResult,
	initiator_contract::{SCIResult, SmartContractInitiatorEvent},
	types::LockDetails,
};

use crate::types::{CompletedDetails, EthAddress};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveCounterpartyEvent<A, H> {
	LockedBridgeTransfer(LockDetails<A, H>),
	CompletedBridgeTransfer(CompletedDetails<A, H>),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MoveCounterpartyError {
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EthInitiatorError {
	#[error("Failed to initiate bridge transfer")]
	InitiateTransferError,
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthChainEvent<A, H> {
	InitiatorContractEvent(SCIResult<A, H>),
	CounterpartyContractEvent(SCCResult<A, H>),
	Noop,
}

impl From<BridgeContractInitiatorEvent<EthAddress, [u8; 32]>>
	for EthChainEvent<EthAddress, [u8; 32]>
{
	fn from(event: BridgeContractInitiatorEvent<EthAddress, [u8; 32]>) -> Self {
		match event {
			BridgeContractInitiatorEvent::Initiated(details) => {
				EthChainEvent::InitiatorContractEvent(Ok(
					SmartContractInitiatorEvent::InitiatedBridgeTransfer(details),
				))
			}
			BridgeContractInitiatorEvent::Completed(id) => EthChainEvent::InitiatorContractEvent(
				Ok(SmartContractInitiatorEvent::CompletedBridgeTransfer(id)),
			),
			_ => unimplemented!(), // Refunded variant
		}
	}
}
