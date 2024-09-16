use bridge_shared::types::{CounterpartyCompletedDetails, LockDetails};
use thiserror::Error;

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveCounterpartyEvent<A, H> {
	LockedBridgeTransfer(LockDetails<A, H>),
	CompletedBridgeTransfer(CounterpartyCompletedDetails<A, H>),
}

#[allow(unused)]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MoveCounterpartyError {
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}
