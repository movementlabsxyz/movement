use thiserror::Error;

use crate::types::{
	Amount, BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId, HashLock,
	HashLockPreImage, InitiatorAddress, RecipientAddress,
};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BridgeContractInitiatorError {
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
	#[error("Initiator address not set")]
	InitiatorAddressNotSet,
	#[error("Failed to convert")]
	ConversionError,
	#[error("Generic error: {0}")]
	GenericError(String),
}

impl BridgeContractInitiatorError {
	pub fn generic<E: std::error::Error>(e: E) -> Self {
		Self::GenericError(e.to_string())
	}
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BridgeContractCounterpartyError {
	#[error("Invalid response length")]
	InvalidResponseLength,
	#[error("Function call failed")]
	CallError,
	#[error("Failed to view module")]
	ModuleViewError,
	#[error("Failed to view function")]
	FunctionViewError,
	#[error("Failed to serialize view args")]
	ViewSerializationError,
	#[error("Failed to serialize or deserialize")]
	SerializationError,
	#[error("Failed to lock bridge transfer")]
	LockTransferError,
	#[error("Failed to complete bridge transfer")]
	CompleteTransferError,
	#[error("Failed to abort bridge transfer")]
	AbortTransferError,
	#[error("Counterparty address not set")]
	CounterpartyAddressNotSet,
	#[error("Error getting the signer")]
	SignerError,
	#[error("Failed to convert")]
	ConversionError,
	#[error("Generic error: {0}")]
	GenericError(String),
}

impl BridgeContractCounterpartyError {
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

pub type BridgeContractInitiatorResult<T> = Result<T, BridgeContractInitiatorError>;
pub type BridgeContractCounterpartyResult<T> = Result<T, BridgeContractCounterpartyError>;
pub type BridgeContractWETH9Result<T> = Result<T, BridgeContractWETH9Error>;

#[async_trait::async_trait]
pub trait BridgeContractInitiator: Clone + Unpin + Send + Sync {
	type Address: BridgeAddressType;
	type Hash: BridgeHashType;

	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Vec<u8>>,
		hash_lock: HashLock<Self::Hash>,
		amount: Amount,
	) -> BridgeContractInitiatorResult<()>;

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractInitiatorResult<()>;

	async fn refund_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<()>;

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>>;
}

#[async_trait::async_trait]
pub trait BridgeContractCounterparty: Clone + Unpin + Send + Sync {
	type Address: BridgeAddressType;
	type Hash: BridgeHashType;

	async fn lock_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		initiator: InitiatorAddress<Vec<u8>>,
		recipient: RecipientAddress<Self::Address>,
		amount: Amount,
	) -> BridgeContractCounterpartyResult<()>;

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()>;

	async fn abort_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()>;

	async fn get_bridge_transfer_details(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>>;
}

#[async_trait::async_trait]
pub trait BridgeContractWETH9: Clone + Unpin + Send + Sync {
	type Address: BridgeAddressType;
	type Hash: BridgeHashType;

	async fn deposit_weth(&mut self, amount: Amount) -> BridgeContractWETH9Result<()>;
}
