use crate::types::{BridgeTransferDetailsCounterparty, LockDetails};
use std::fmt;
use thiserror::Error;
use tokio_stream::Stream;

use crate::types::{
	Amount, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BridgeContractError {
	#[error("Failed to extract transfer Id")]
	TransferIdExtractionError,
	#[error("Failed to mint")]
	MintError,
	#[error("Failed to call function")]
	CallError,
	#[error("Failed to serialize or deserialize")]
	SerializationError,
	#[error("Invalid response length")]
	InvalidResponseLength,
	#[error("Failed to view function")]
	FunctionViewError,
	#[error("Failed to initiate bridge transfer")]
	InitiateTransferError,
	#[error("Failed to complete bridge transfer")]
	CompleteTransferError,
	#[error("Failed to parse preimage")]
	ParsePreimageError,
	#[error("Contract address parse error")]
	ContractAddressError,
	#[error("Failed to convert:{0}")]
	ConversionFailed(String),
	#[error("Generic error: {0}")]
	GenericError(String),
	#[error("Failed to view module")]
	ModuleViewError,
	#[error("Failed to serialize view args")]
	ViewSerializationError,
	#[error("Failed to lock bridge transfer")]
	LockTransferError,
	#[error("Failed to abort bridge transfer")]
	AbortTransferError,
	#[error("Address not set")]
	AddressNotSet,
	#[error("Error getting the signer")]
	SignerError,
	#[error("Error received an unknown onchain event")]
	OnChainUnknownEvent,
	#[error("Error during onchain call:{0}")]
	OnChainError(String),
	#[error("Error during deserializing an event :{1:?} : {0}")]
	EventDeserializingFail(String, BridgeContractEventType),
}

impl BridgeContractError {
	pub fn generic<E: std::error::Error>(e: E) -> Self {
		Self::GenericError(e.to_string())
	}
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BridgeContractWETH9Error {
	#[error("Insufficient balance")]
	BalanceError,
	#[error("Allowance exceeded")]
	AllowanceError,
	#[error("Generic error: {0}")]
	GenericError(String),
}
impl BridgeContractWETH9Error {
	pub fn generic<E: std::error::Error>(e: E) -> Self {
		Self::GenericError(e.to_string())
	}
}

pub type BridgeContractResult<T> = Result<T, BridgeContractError>;
pub type BridgeContractWETH9Result<T> = Result<T, BridgeContractWETH9Error>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeContractEventType {
	Initiated,
	Locked,
	InitialtorCompleted,
	CounterPartCompleted,
	Cancelled,
	Refunded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeContractEvent<A> {
	Initiated(BridgeTransferDetails<A>),
	Locked(LockDetails<A>),
	InitialtorCompleted(BridgeTransferId),
	CounterPartCompleted(BridgeTransferId, HashLockPreImage),
	Cancelled(BridgeTransferId),
	Refunded(BridgeTransferId),
}

impl<A> BridgeContractEvent<A> {
	pub fn bridge_transfer_id(&self) -> BridgeTransferId {
		match self {
			Self::Initiated(details) => details.bridge_transfer_id,
			Self::Locked(details) => details.bridge_transfer_id,
			Self::InitialtorCompleted(id)
			| Self::CounterPartCompleted(id, _)
			| Self::Cancelled(id)
			| Self::Refunded(id) => *id,
		}
	}

	pub fn is_initiated_event(&self) -> bool {
		if let BridgeContractEvent::Initiated(_) = self {
			true
		} else {
			false
		}
	}
}

impl<A> fmt::Display for BridgeContractEvent<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let kind = match self {
			Self::Initiated(_) => "Initiated",
			Self::Locked(_) => "Locked",
			Self::InitialtorCompleted(_) => "InitialtorCompleted",
			Self::CounterPartCompleted(_, _) => "CounterPartCompleted",
			Self::Cancelled(_) => "Cancelled",
			Self::Refunded(_) => "Refunded",
		};
		write!(f, "Contract event: {}/ transfer id: {}", kind, self.bridge_transfer_id(),)
	}
}

pub trait BridgeContractMonitoring:
	Stream<Item = BridgeContractResult<BridgeContractEvent<Self::Address>>> + Unpin
{
	type Address;
}

#[async_trait::async_trait]
pub trait BridgeContract<A>: Clone + Unpin + Send + Sync {
	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: BridgeAddress<A>,
		recipient_address: BridgeAddress<Vec<u8>>,
		hash_lock: HashLock,
		amount: Amount,
	) -> BridgeContractResult<()>;

	async fn initiator_complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		secret: HashLockPreImage,
	) -> BridgeContractResult<()>;

	async fn counterparty_complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		secret: HashLockPreImage,
	) -> BridgeContractResult<()>;

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<()>;

	async fn get_bridge_transfer_details_initiator(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferDetails<A>>>;

	async fn get_bridge_transfer_details_counterparty(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferDetailsCounterparty<A>>>;

	async fn lock_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		hash_lock: HashLock,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<A>,
		amount: Amount,
	) -> BridgeContractResult<()>;

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<()>;
}

#[async_trait::async_trait]
pub trait BridgeContractWETH9: Clone + Unpin + Send + Sync {
	async fn deposit_weth(&mut self, amount: Amount) -> BridgeContractWETH9Result<()>;
}
