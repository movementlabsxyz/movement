use crate::types::{
	Amount, BridgeAddress, BridgeTransferCompletedDetails, BridgeTransferId,
	BridgeTransferInitiatedDetails, Nonce,
};
use std::fmt;
use thiserror::Error;
use tokio_stream::Stream;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BridgeContractError {
	#[error("Account balance error")]
	AccountBalanceError,
	#[error("Funding error")]
	FundingError,
	#[error("Invalid Url")]
	InvalidUrl,
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
	#[error("Error during decoding address:{0}")]
	BadAddressEncoding(String),
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
	Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeContractEvent<A> {
	Initiated(BridgeTransferInitiatedDetails<A>),
	Completed(BridgeTransferCompletedDetails<A>),
}

impl<A> BridgeContractEvent<A> {
	pub fn bridge_transfer_id(&self) -> BridgeTransferId {
		match self {
			Self::Initiated(details) => details.bridge_transfer_id,
			Self::Completed(details) => details.bridge_transfer_id,
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
			Self::Completed(_) => "Completed",
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
pub trait BridgeClientContract<A>: Clone + Unpin + Send + Sync {
	async fn initiate_bridge_transfer(
		&mut self,
		recipient: BridgeAddress<Vec<u8>>,
		amount: Amount,
	) -> BridgeContractResult<()>;

	async fn get_bridge_transfer_details_initiate(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferInitiatedDetails<A>>>;

	async fn get_bridge_transfer_details_complete(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
	) -> BridgeContractResult<Option<BridgeTransferCompletedDetails<A>>>;
}

#[async_trait::async_trait]
pub trait BridgeRelayerContract<A>: Clone + Unpin + Send + Sync {
	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId,
		initiator: BridgeAddress<Vec<u8>>,
		recipient: BridgeAddress<A>,
		amount: Amount,
		nonce: Nonce,
	) -> BridgeContractResult<()>;
}

#[async_trait::async_trait]
pub trait BridgeContractWETH9: Clone + Unpin + Send + Sync {
	async fn deposit_weth(&mut self, amount: Amount) -> BridgeContractWETH9Result<()>;
}
