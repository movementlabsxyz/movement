use crate::types::EthAddress;
use bridge_shared::bridge_monitoring::{
	BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent,
};
use bridge_shared::types::{
	SCCResult, SCIResult, SmartContractCounterpartyEvent, SmartContractInitiatorEvent,
};
use thiserror::Error;

#[allow(unused)]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EthInitiatorError {
	#[error("Failed to initiate bridge transfer")]
	InitiateTransferError,
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[allow(unused)]
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
			BridgeContractInitiatorEvent::Refunded(id) => EthChainEvent::InitiatorContractEvent(
				Ok(SmartContractInitiatorEvent::RefundedBridgeTransfer(id)),
			),
		}
	}
}

impl From<BridgeContractCounterpartyEvent<EthAddress, [u8; 32]>>
	for EthChainEvent<EthAddress, [u8; 32]>
{
	fn from(event: BridgeContractCounterpartyEvent<EthAddress, [u8; 32]>) -> Self {
		match event {
			BridgeContractCounterpartyEvent::Locked(details) => {
				EthChainEvent::CounterpartyContractEvent(Ok(
					SmartContractCounterpartyEvent::LockedBridgeTransfer(details),
				))
			}
			BridgeContractCounterpartyEvent::Completed(details) => {
				EthChainEvent::CounterpartyContractEvent(Ok(
					SmartContractCounterpartyEvent::CompletedBridgeTransfer(details),
				))
			}
		}
	}
}
