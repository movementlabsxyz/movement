use thiserror::Error;

use crate::types::{BridgeTransferDetails, BridgeTransferId};

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

pub type BridgeContractResult<T> = Result<T, BridgeContractError>;

#[async_trait::async_trait]
pub trait BridgeContractInitiator {
	type Address;
	type Hash;

	async fn initiate_bridge_transfer(
		&self,
		initiator_address: Self::Address,
		recipient_address: Self::Address,
		hash_lock: Self::Hash,
		time_lock: u64,
		amount: u64,
	) -> BridgeContractResult<()>;

	async fn complete_bridge_transfer<S: Send>(
		&self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: S,
	) -> BridgeContractResult<()>;

	async fn refund_bridge_transfer(
		&self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()>;

	async fn get_bridge_transfer_details(
		&self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>;
}

#[async_trait::async_trait]
pub trait BridgeContractCounterparty {
	type Address;
	type Hash;

	async fn lock_bridge_transfer_assets(
		&self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: Self::Hash,
		time_lock: u64,
		recipient: Self::Address,
		amount: u64,
	) -> bool;

	async fn complete_bridge_transfer<S: Send>(
		&self,
		bridge_transfer_id: Self::Hash,
		secret: S,
	) -> BridgeContractResult<()>;

	async fn abort_bridge_transfer(
		&self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()>;

	async fn get_bridge_transfer_details(
		&self,
		bridge_transfer_id: Self::Hash,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>;
}
