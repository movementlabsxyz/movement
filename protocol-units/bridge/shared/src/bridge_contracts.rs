use thiserror::Error;

use crate::types::{
	Amount, BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId, HashLock,
	HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock,
};

#[derive(Error, Debug)]
pub enum BridgeContractError {
	#[error("Failed to initiate bridge transfer")]
	InitiateTransferError,
	#[error("Failed to complete bridge transfer")]
	CompleteTransferError,
	#[error("Event monitoring error")]
	EventMonitorError,
	#[error("Generic error: {0}")]
	GenericError(String),
}

impl BridgeContractError {
	pub fn generic<E: std::error::Error>(e: E) -> Self {
		Self::GenericError(e.to_string())
	}
}

pub type BridgeContractResult<T> = Result<T, BridgeContractError>;

#[async_trait::async_trait]
pub trait BridgeContractInitiator: Clone + Unpin + Send + Sync {
	type Address: BridgeAddressType;
	type Hash: BridgeHashType;

	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Self::Address>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractResult<()>;

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractResult<()>;

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()>;

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>;
}

#[async_trait::async_trait]
pub trait BridgeContractCounterparty: Clone + Unpin + Send + Sync {
	type Address: BridgeAddressType;
	type Hash: BridgeHashType;

	async fn lock_bridge_transfer_assets(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		recipient: RecipientAddress<Self::Address>,
		amount: Amount,
	) -> bool;

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractResult<()>;

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()>;

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>;
}
