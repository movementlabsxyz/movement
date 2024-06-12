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
pub trait BridgeContractInitiator<A, H> {
	async fn initiate_bridge_transfer(
		&self,
		initiator_address: A,
		recipient_address: A,
		hash_lock: H,
		time_lock: u64,
		amount: u64,
	) -> BridgeContractResult<()>;

	async fn complete_bridge_transfer<S: Send>(
		&self,
		bridge_transfer_id: BridgeTransferId<H>,
		secret: S,
	) -> BridgeContractResult<()>;

	async fn refund_bridge_transfer(
		&self,
		bridge_transfer_id: BridgeTransferId<H>,
	) -> BridgeContractResult<()>;

	async fn get_bridge_transfer_details(
		&self,
		bridge_transfer_id: BridgeTransferId<H>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<H, A>>>;
}

#[async_trait::async_trait]
pub trait BridgeContractCounterparty<A, H> {
	async fn lock_bridge_transfer_assets(
		&self,
		bridge_transfer_id: String,
		hash_lock: String,
		time_lock: u64,
		recipient: String,
		amount: u64,
	) -> bool;

	async fn complete_bridge_transfer<S: Send>(
		&self,
		bridge_transfer_id: H,
		secret: S,
	) -> BridgeContractResult<()>;

	async fn abort_bridge_transfer(
		&self,
		bridge_transfer_id: BridgeTransferId<H>,
	) -> BridgeContractResult<()>;

	async fn get_bridge_transfer_details(
		&self,
		bridge_transfer_id: H,
	) -> BridgeContractResult<Option<BridgeTransferDetails<H, A>>>;
}
