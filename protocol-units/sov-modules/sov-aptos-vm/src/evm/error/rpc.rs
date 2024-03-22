//! Implementation-specific Errors for the `eth_` namespace.

use std::convert::Infallible;
use std::time::Duration;

use alloy_sol_types::decode_revert_reason;
use reth_rpc_types::request::TransactionInputError;
use revm::primitives::{Address, EVMError, HaltReason, InvalidHeader, U256};

use super::pool::{
    Eip4844PoolTransactionError, InvalidPoolTransactionError, PoolError, PoolTransactionError,
};

/// Result alias
pub type EthResult<T> = Result<T, EthApiError>;

/// Errors that can occur when interacting with the `eth_` namespace
#[derive(Debug, thiserror::Error)]
pub enum EthApiError {
    /// When a raw transaction is empty
    #[error("Empty transaction data")]
    EmptyRawTransactionData,
    /// When decoding a signed transaction fails
    #[error("Failed to decode signed transaction")]
    FailedToDecodeSignedTransaction,
    /// When the transaction signature is invalid
    #[error("Invalid transaction signature")]
    InvalidTransactionSignature,
    /// Errors related to the transaction pool
    #[error(transparent)]
    PoolError(RpcPoolError),
    /// When an unknown block number is encountered
    #[error("Unknown block number")]
    UnknownBlockNumber,
    /// Thrown when querying for `finalized` or `safe` block before the merge transition is
    /// finalized, <https://github.com/ethereum/execution-apis/blob/6d17705a875e52c26826124c2a8a15ed542aeca2/src/schemas/block.yaml#L109>
    #[error("Unknown block")]
    UnknownSafeOrFinalizedBlock,
    /// Thrown when an unknown block or transaction index is encountered
    #[error("unknown block or tx index")]
    UnknownBlockOrTxIndex,
    /// When an invalid block range is provided
    #[error("Invalid block range")]
    InvalidBlockRange,
    /// An internal error where prevrandao is not set in the evm's environment
    #[error("Prevrandao not in the EVM's environment after merge")]
    PrevrandaoNotSet,
    /// `excess_blob_gas` is not set for Cancun and above
    #[error("Excess blob gas missing in the EVM's environment after Cancun")]
    ExcessBlobGasNotSet,
    /// Thrown when a call or transaction request (`eth_call`, `eth_estimateGas`,
    /// `eth_sendTransaction`) contains conflicting fields (legacy, EIP-1559)
    #[error("Both gasPrice and (maxFeePerGas or maxPriorityFeePerGas) specified")]
    ConflictingFeeFieldsInRequest,
    /// Errors related to invalid transactions
    #[error(transparent)]
    InvalidTransaction(#[from] RpcInvalidTransactionError),
    /// Thrown when constructing an RPC block from primitive block data fails
    #[error(transparent)]
    InvalidBlockData(#[from] reth_rpc_types::BlockError),
    /// Thrown when an `AccountOverride` contains conflicting `state` and `stateDiff` fields
    #[error("Account {0:?} has both 'state' and 'stateDiff'")]
    BothStateAndStateDiffInOverride(Address),
    /// Other internal error
    #[cfg(feature = "native")]
    #[error(transparent)]
    Internal(reth_interfaces::RethError),
    /// Error related to signing
    #[error(transparent)]
    Signing(#[from] SignError),
    /// Thrown when a transaction was requested but not matching transaction exists
    #[error("transaction not found")]
    TransactionNotFound,
    /// Some feature is unsupported
    #[error("unsupported")]
    Unsupported(&'static str),
    /// General purpose error for invalid params
    #[error("{0}")]
    InvalidParams(String),
    /// When the tracer config does not match the tracer
    #[error("invalid tracer config")]
    InvalidTracerConfig,
    /// When the percentile array is invalid
    #[error("invalid reward percentiles")]
    InvalidRewardPercentiles,
    /// Error thrown when a spawned blocking task failed to deliver an anticipated response.
    ///
    /// This only happens if the blocking task panics and is aborted before it can return a
    /// response back to the request handler.
    #[error("internal blocking task error")]
    InternalBlockingTaskError,
    /// Error thrown when a spawned blocking task failed to deliver an anticipated response
    #[error("internal eth error")]
    InternalEthError,
    /// Error thrown when a (tracing) call exceeds the configured timeout
    #[error("execution aborted (timeout = {0:?})")]
    ExecutionTimedOut(Duration),
    /// Internal Error thrown by the javascript tracer
    #[error("{0}")]
    InternalJsTracerError(String),
    #[error(transparent)]
    /// Call Input error when both `data` and `input` fields are set and not equal.
    TransactionInputError(#[from] TransactionInputError),
    /// Evm generic purpose error.
    #[error("Revm error: {0}")]
    EvmCustom(String),
}

#[cfg(feature = "native")]
impl From<EthApiError> for jsonrpsee::types::ErrorObject<'static> {
    fn from(error: EthApiError) -> Self {
        match error {
            EthApiError::FailedToDecodeSignedTransaction
            | EthApiError::InvalidTransactionSignature
            | EthApiError::EmptyRawTransactionData
            | EthApiError::InvalidBlockRange
            | EthApiError::ConflictingFeeFieldsInRequest
            | EthApiError::Signing(_)
            | EthApiError::BothStateAndStateDiffInOverride(_)
            | EthApiError::InvalidTracerConfig => invalid_params_rpc_err(error.to_string()),
            EthApiError::InvalidTransaction(err) => err.into(),
            EthApiError::PoolError(err) => err.into(),
            EthApiError::PrevrandaoNotSet
            | EthApiError::ExcessBlobGasNotSet
            | EthApiError::InvalidBlockData(_)
            | EthApiError::Internal(_)
            | EthApiError::TransactionNotFound
            | EthApiError::EvmCustom(_) => internal_rpc_err(error.to_string()),
            EthApiError::UnknownBlockNumber | EthApiError::UnknownBlockOrTxIndex => {
                rpc_error_with_code(
                    reth_rpc_types::error::EthRpcErrorCode::ResourceNotFound.code(),
                    error.to_string(),
                )
            }
            EthApiError::UnknownSafeOrFinalizedBlock => rpc_error_with_code(
                reth_rpc_types::error::EthRpcErrorCode::UnknownBlock.code(),
                error.to_string(),
            ),
            EthApiError::Unsupported(msg) => internal_rpc_err(msg),
            EthApiError::InternalJsTracerError(msg) => internal_rpc_err(msg),
            EthApiError::InvalidParams(msg) => invalid_params_rpc_err(msg),
            EthApiError::InvalidRewardPercentiles => internal_rpc_err(error.to_string()),
            err @ EthApiError::ExecutionTimedOut(_) => rpc_error_with_code(
                jsonrpsee::types::error::CALL_EXECUTION_FAILED_CODE,
                err.to_string(),
            ),
            err @ EthApiError::InternalBlockingTaskError => internal_rpc_err(err.to_string()),
            err @ EthApiError::InternalEthError => internal_rpc_err(err.to_string()),
            err @ EthApiError::TransactionInputError(_) => invalid_params_rpc_err(err.to_string()),
        }
    }
}

#[cfg(feature = "native")]
impl From<EthApiError> for jsonrpsee::core::Error {
    fn from(error: EthApiError) -> Self {
        jsonrpsee::core::Error::Call(error.into())
    }
}

#[cfg(feature = "native")]
impl From<reth_interfaces::RethError> for EthApiError {
    fn from(error: reth_interfaces::RethError) -> Self {
        match error {
            reth_interfaces::RethError::Provider(err) => err.into(),
            err => EthApiError::Internal(err),
        }
    }
}

#[cfg(feature = "native")]
impl From<reth_interfaces::provider::ProviderError> for EthApiError {
    fn from(error: reth_interfaces::provider::ProviderError) -> Self {
        use reth_interfaces::provider::ProviderError;
        match error {
            ProviderError::HeaderNotFound(_)
            | ProviderError::BlockHashNotFound(_)
            | ProviderError::BestBlockNotFound
            | ProviderError::BlockNumberForTransactionIndexNotFound
            | ProviderError::TotalDifficultyNotFound { .. }
            | ProviderError::UnknownBlockHash(_) => EthApiError::UnknownBlockNumber,
            ProviderError::FinalizedBlockNotFound | ProviderError::SafeBlockNotFound => {
                EthApiError::UnknownSafeOrFinalizedBlock
            }
            err => EthApiError::Internal(err.into()),
        }
    }
}

impl From<EVMError<Infallible>> for EthApiError {
    fn from(err: EVMError<Infallible>) -> Self {
        match err {
            EVMError::Transaction(err) => RpcInvalidTransactionError::from(err).into(),
            EVMError::Header(InvalidHeader::PrevrandaoNotSet) => EthApiError::PrevrandaoNotSet,
            EVMError::Header(InvalidHeader::ExcessBlobGasNotSet) => {
                EthApiError::ExcessBlobGasNotSet
            }
            EVMError::Database(_) => {
                panic!("Infallible error triggered")
            }
            EVMError::Custom(data) => EthApiError::EvmCustom(data),
        }
    }
}

/// An error due to invalid transaction.
///
/// The only reason this exists is to maintain compatibility with other clients de-facto standard
/// error messages.
///
/// These error variants can be thrown when the transaction is checked prior to execution.
///
/// These variants also cover all errors that can be thrown by revm.
///
/// ## Nomenclature
///
/// This type is explicitly modelled after geth's error variants and uses
///   `fee cap` for `max_fee_per_gas`
///   `tip` for `max_priority_fee_per_gas`
#[derive(thiserror::Error, Debug)]
pub enum RpcInvalidTransactionError {
    /// returned if the nonce of a transaction is lower than the one present in the local chain.
    #[error("nonce too low")]
    NonceTooLow,
    /// returned if the nonce of a transaction is higher than the next one expected based on the
    /// local chain.
    #[error("nonce too high")]
    NonceTooHigh,
    /// Returned if the nonce of a transaction is too high.
    /// Incrementing the nonce would lead to invalid state (overflow).
    #[error("nonce has max value")]
    NonceMaxValue,
    /// Thrown if the transaction sender doesn't have enough funds for a transfer.
    #[error("insufficient funds for transfer")]
    InsufficientFundsForTransfer,
    /// thrown if creation transaction provides the init code bigger than init code size limit.
    #[error("max initcode size exceeded")]
    MaxInitCodeSizeExceeded,
    /// Represents the inability to cover max cost + value (account balance too low).
    #[error("insufficient funds for gas * price + value")]
    InsufficientFunds,
    /// Thrown when calculating gas usage
    #[error("gas uint64 overflow")]
    GasUintOverflow,
    /// returned if the transaction is specified to use less gas than required to start the
    /// invocation.
    #[error("intrinsic gas too low")]
    GasTooLow,
    /// returned if the transaction gas exceeds the limit
    #[error("intrinsic gas too high")]
    GasTooHigh,
    /// thrown if a transaction is not supported in the current network configuration.
    #[error("transaction type not supported")]
    TxTypeNotSupported,
    /// Thrown to ensure no one is able to specify a transaction with a tip higher than the total
    /// fee cap.
    #[error("max priority fee per gas higher than max fee per gas")]
    TipAboveFeeCap,
    /// A sanity error to avoid huge numbers specified in the tip field.
    #[error("max priority fee per gas higher than 2^256-1")]
    TipVeryHigh,
    /// A sanity error to avoid huge numbers specified in the fee cap field.
    #[error("max fee per gas higher than 2^256-1")]
    FeeCapVeryHigh,
    /// Thrown post London if the transaction's fee is less than the base fee of the block
    #[error("max fee per gas less than block base fee")]
    FeeCapTooLow,
    /// Thrown if the sender of a transaction is a contract.
    #[error("sender not an eoa")]
    SenderNoEOA,
    /// Thrown during estimate if caller has insufficient funds to cover the tx.
    #[error("Out of gas: gas required exceeds allowance: {0:?}")]
    BasicOutOfGas(U256),
    /// As BasicOutOfGas but thrown when gas exhausts during memory expansion.
    #[error("Out of gas: gas exhausts during memory expansion: {0:?}")]
    MemoryOutOfGas(U256),
    /// As BasicOutOfGas but thrown when gas exhausts during precompiled contract execution.
    #[error("Out of gas: gas exhausts during precompiled contract execution: {0:?}")]
    PrecompileOutOfGas(U256),
    /// revm's Type cast error, U256 casts down to an u64 with overflow
    #[error("Out of gas: revm's Type cast error, U256 casts down to a u64 with overflow {0:?}")]
    InvalidOperandOutOfGas(U256),
    /// Thrown if executing a transaction failed during estimate/call
    #[error("{0}")]
    Revert(RevertError),
    /// Unspecific evm halt error
    #[error("EVM error {0:?}")]
    EvmHalt(HaltReason),
    /// Invalid chain id set for the transaction.
    #[error("Invalid chain id")]
    InvalidChainId,
    /// The transaction is before Spurious Dragon and has a chain ID
    #[error("Transactions before Spurious Dragon should not have a chain ID.")]
    OldLegacyChainId,
    /// The transitions is before Berlin and has access list
    #[error("Transactions before Berlin should not have access list")]
    AccessListNotSupported,
    /// `max_fee_per_blob_gas` is not supported for blocks before the Cancun hard fork.
    #[error("max_fee_per_blob_gas is not supported for blocks before the Cancun hard fork.")]
    MaxFeePerBlobGasNotSupported,
    /// `blob_hashes`/`blob_versioned_hashes` is not supported for blocks before the Cancun
    /// hardfork.
    #[error("blob_versioned_hashes is not supported for blocks before the Cancun hard fork.")]
    BlobVersionedHashesNotSupported,
    /// Block `blob_gas_price` is greater than tx-specified `max_fee_per_blob_gas` after Cancun.
    #[error("max fee per blob gas less than block blob gas fee")]
    BlobFeeCapTooLow,
    /// Blob transaction has a versioned hash with an invalid blob
    #[error("blob hash version mismatch")]
    BlobHashVersionMismatch,
    /// Blob transaction has no versioned hashes
    #[error("blob transaction missing blob hashes")]
    BlobTransactionMissingBlobHashes,
    /// Blob transaction has too many blobs
    #[error("blob transaction exceeds max blobs per block")]
    TooManyBlobs,
    /// Blob transaction is a create transaction
    #[error("blob transaction is a create transaction")]
    BlobTransactionIsCreate,
}

#[cfg(feature = "native")]
impl RpcInvalidTransactionError {
    /// Returns the rpc error code for this error.
    fn error_code(&self) -> i32 {
        match self {
            RpcInvalidTransactionError::InvalidChainId
            | RpcInvalidTransactionError::GasTooLow
            | RpcInvalidTransactionError::GasTooHigh => {
                reth_rpc_types::error::EthRpcErrorCode::InvalidInput.code()
            }
            RpcInvalidTransactionError::Revert(_) => {
                reth_rpc_types::error::EthRpcErrorCode::ExecutionError.code()
            }
            _ => reth_rpc_types::error::EthRpcErrorCode::TransactionRejected.code(),
        }
    }

    /// Converts the halt error
    ///
    /// Takes the configured gas limit of the transaction which is attached to the error
    pub(crate) fn halt(reason: HaltReason, gas_limit: u64) -> Self {
        match reason {
            HaltReason::OutOfGas(err) => RpcInvalidTransactionError::out_of_gas(err, gas_limit),
            HaltReason::NonceOverflow => RpcInvalidTransactionError::NonceMaxValue,
            err => RpcInvalidTransactionError::EvmHalt(err),
        }
    }

    /// Converts the out of gas error
    pub(crate) fn out_of_gas(reason: revm::primitives::OutOfGasError, gas_limit: u64) -> Self {
        let gas_limit = U256::from(gas_limit);
        match reason {
            revm::primitives::OutOfGasError::Basic => {
                RpcInvalidTransactionError::BasicOutOfGas(gas_limit)
            }
            revm::primitives::OutOfGasError::Memory => {
                RpcInvalidTransactionError::MemoryOutOfGas(gas_limit)
            }
            revm::primitives::OutOfGasError::Precompile => {
                RpcInvalidTransactionError::PrecompileOutOfGas(gas_limit)
            }
            revm::primitives::OutOfGasError::InvalidOperand => {
                RpcInvalidTransactionError::InvalidOperandOutOfGas(gas_limit)
            }
            revm::primitives::OutOfGasError::MemoryLimit => {
                RpcInvalidTransactionError::MemoryOutOfGas(gas_limit)
            }
        }
    }
}

#[cfg(feature = "native")]
impl From<RpcInvalidTransactionError> for jsonrpsee::types::ErrorObject<'static> {
    fn from(err: RpcInvalidTransactionError) -> Self {
        match err {
            RpcInvalidTransactionError::Revert(revert) => {
                // include out data if some
                rpc_err(
                    revert.error_code(),
                    revert.to_string(),
                    revert.output.as_ref().map(|out| out.as_ref()),
                )
            }
            err => rpc_err(err.error_code(), err.to_string(), None),
        }
    }
}

impl From<revm::primitives::InvalidTransaction> for RpcInvalidTransactionError {
    fn from(err: revm::primitives::InvalidTransaction) -> Self {
        use revm::primitives::InvalidTransaction;
        match err {
            InvalidTransaction::InvalidChainId => RpcInvalidTransactionError::InvalidChainId,
            InvalidTransaction::PriorityFeeGreaterThanMaxFee => {
                RpcInvalidTransactionError::TipAboveFeeCap
            }
            InvalidTransaction::GasPriceLessThanBasefee => RpcInvalidTransactionError::FeeCapTooLow,
            InvalidTransaction::CallerGasLimitMoreThanBlock => {
                RpcInvalidTransactionError::GasTooHigh
            }
            InvalidTransaction::CallGasCostMoreThanGasLimit => {
                RpcInvalidTransactionError::GasTooHigh
            }
            InvalidTransaction::RejectCallerWithCode => RpcInvalidTransactionError::SenderNoEOA,
            InvalidTransaction::LackOfFundForMaxFee { .. } => {
                RpcInvalidTransactionError::InsufficientFunds
            }
            InvalidTransaction::OverflowPaymentInTransaction => {
                RpcInvalidTransactionError::GasUintOverflow
            }
            InvalidTransaction::NonceOverflowInTransaction => {
                RpcInvalidTransactionError::NonceMaxValue
            }
            InvalidTransaction::CreateInitCodeSizeLimit => {
                RpcInvalidTransactionError::MaxInitCodeSizeExceeded
            }
            InvalidTransaction::NonceTooHigh { .. } => RpcInvalidTransactionError::NonceTooHigh,
            InvalidTransaction::NonceTooLow { .. } => RpcInvalidTransactionError::NonceTooLow,
            InvalidTransaction::AccessListNotSupported => {
                RpcInvalidTransactionError::AccessListNotSupported
            }
            InvalidTransaction::MaxFeePerBlobGasNotSupported => {
                RpcInvalidTransactionError::MaxFeePerBlobGasNotSupported
            }
            InvalidTransaction::BlobVersionedHashesNotSupported => {
                RpcInvalidTransactionError::BlobVersionedHashesNotSupported
            }
            InvalidTransaction::BlobGasPriceGreaterThanMax => {
                RpcInvalidTransactionError::BlobFeeCapTooLow
            }
            InvalidTransaction::EmptyBlobs => {
                RpcInvalidTransactionError::BlobTransactionMissingBlobHashes
            }
            InvalidTransaction::BlobVersionNotSupported => {
                RpcInvalidTransactionError::BlobHashVersionMismatch
            }
            InvalidTransaction::TooManyBlobs => RpcInvalidTransactionError::TooManyBlobs,
            InvalidTransaction::BlobCreateTransaction => {
                RpcInvalidTransactionError::BlobTransactionIsCreate
            }
            _ => panic!("InvalidTransaction error not handled: {:?}", err),
        }
    }
}

impl From<reth_primitives::InvalidTransactionError> for RpcInvalidTransactionError {
    fn from(err: reth_primitives::InvalidTransactionError) -> Self {
        use reth_primitives::InvalidTransactionError;
        // This conversion is used to convert any transaction errors that could occur inside the
        // txpool (e.g. `eth_sendRawTransaction`) to their corresponding RPC
        match err {
            InvalidTransactionError::InsufficientFunds { .. } => {
                RpcInvalidTransactionError::InsufficientFunds
            }
            InvalidTransactionError::NonceNotConsistent => RpcInvalidTransactionError::NonceTooLow,
            InvalidTransactionError::OldLegacyChainId => {
                // Note: this should be unreachable since Spurious Dragon now enabled
                RpcInvalidTransactionError::OldLegacyChainId
            }
            InvalidTransactionError::ChainIdMismatch => RpcInvalidTransactionError::InvalidChainId,
            InvalidTransactionError::Eip2930Disabled
            | InvalidTransactionError::Eip1559Disabled
            | InvalidTransactionError::Eip4844Disabled => {
                RpcInvalidTransactionError::TxTypeNotSupported
            }
            InvalidTransactionError::TxTypeNotSupported => {
                RpcInvalidTransactionError::TxTypeNotSupported
            }
            InvalidTransactionError::GasUintOverflow => RpcInvalidTransactionError::GasUintOverflow,
            InvalidTransactionError::GasTooLow => RpcInvalidTransactionError::GasTooLow,
            InvalidTransactionError::GasTooHigh => RpcInvalidTransactionError::GasTooHigh,
            InvalidTransactionError::TipAboveFeeCap => RpcInvalidTransactionError::TipAboveFeeCap,
            InvalidTransactionError::FeeCapTooLow => RpcInvalidTransactionError::FeeCapTooLow,
            InvalidTransactionError::SignerAccountHasBytecode => {
                RpcInvalidTransactionError::SenderNoEOA
            }
        }
    }
}

/// Represents a reverted transaction and its output data.
///
/// Displays "execution reverted(: reason)?" if the reason is a string.
#[derive(Debug, Clone)]
pub struct RevertError {
    /// The transaction output data
    ///
    /// Note: this is `None` if output was empty
    output: Option<bytes::Bytes>,
}

// === impl RevertError ==

impl RevertError {
    /// Wraps the output bytes
    ///
    /// Note: this is intended to wrap an revm output
    pub fn new(output: bytes::Bytes) -> Self {
        if output.is_empty() {
            Self { output: None }
        } else {
            Self {
                output: Some(output),
            }
        }
    }

    #[cfg(feature = "native")]
    fn error_code(&self) -> i32 {
        reth_rpc_types::error::EthRpcErrorCode::ExecutionError.code()
    }
}

impl std::fmt::Display for RevertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("execution reverted")?;
        if let Some(reason) = self.output.as_ref().and_then(|r| decode_revert_reason(r)) {
            write!(f, ": {reason}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RevertError {}

/// A helper error type that's mainly used to mirror `geth` Txpool's error messages
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum RpcPoolError {
    #[error("already known")]
    AlreadyKnown,
    #[error("invalid sender")]
    InvalidSender,
    #[error("transaction underpriced")]
    Underpriced,
    #[error("txpool is full")]
    TxPoolOverflow,
    #[error("replacement transaction underpriced")]
    ReplaceUnderpriced,
    #[error("exceeds block gas limit")]
    ExceedsGasLimit,
    #[error("negative value")]
    NegativeValue,
    #[error("oversized data")]
    OversizedData,
    #[error("max initcode size exceeded")]
    ExceedsMaxInitCodeSize,
    #[error(transparent)]
    Invalid(#[from] RpcInvalidTransactionError),
    /// Custom pool error
    #[error("{0:?}")]
    PoolTransactionError(Box<dyn PoolTransactionError>),
    /// Eip-4844 related error
    #[error(transparent)]
    Eip4844(#[from] Eip4844PoolTransactionError),
    /// Thrown if a conflicting transaction type is already in the pool
    ///
    /// In other words, thrown if a transaction with the same sender that violates the exclusivity
    /// constraint (blob vs normal tx)
    #[error("address already reserved")]
    AddressAlreadyReserved,
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "native")]
impl From<RpcPoolError> for jsonrpsee::types::ErrorObject<'static> {
    fn from(error: RpcPoolError) -> Self {
        match error {
            RpcPoolError::Invalid(err) => err.into(),
            error => internal_rpc_err(error.to_string()),
        }
    }
}

impl From<PoolError> for RpcPoolError {
    fn from(err: PoolError) -> RpcPoolError {
        match err {
            PoolError::ReplacementUnderpriced(_) => RpcPoolError::ReplaceUnderpriced,
            PoolError::FeeCapBelowMinimumProtocolFeeCap(_, _) => RpcPoolError::Underpriced,
            PoolError::SpammerExceededCapacity(_, _) => RpcPoolError::TxPoolOverflow,
            PoolError::DiscardedOnInsert(_) => RpcPoolError::TxPoolOverflow,
            PoolError::InvalidTransaction(_, err) => err.into(),
            PoolError::Other(_, err) => RpcPoolError::Other(err),
            PoolError::AlreadyImported(_) => RpcPoolError::AlreadyKnown,
            PoolError::ExistingConflictingTransactionType(_, _, _) => {
                RpcPoolError::AddressAlreadyReserved
            }
        }
    }
}

impl From<InvalidPoolTransactionError> for RpcPoolError {
    fn from(err: InvalidPoolTransactionError) -> RpcPoolError {
        match err {
            InvalidPoolTransactionError::Consensus(err) => RpcPoolError::Invalid(err.into()),
            InvalidPoolTransactionError::ExceedsGasLimit(_, _) => RpcPoolError::ExceedsGasLimit,
            InvalidPoolTransactionError::ExceedsMaxInitCodeSize(_, _) => {
                RpcPoolError::ExceedsMaxInitCodeSize
            }
            InvalidPoolTransactionError::OversizedData(_, _) => RpcPoolError::OversizedData,
            InvalidPoolTransactionError::Underpriced => RpcPoolError::Underpriced,
            InvalidPoolTransactionError::Other(err) => RpcPoolError::PoolTransactionError(err),
            InvalidPoolTransactionError::Eip4844(err) => RpcPoolError::Eip4844(err),
            InvalidPoolTransactionError::Overdraft => {
                RpcPoolError::Invalid(RpcInvalidTransactionError::InsufficientFunds)
            }
        }
    }
}

impl From<PoolError> for EthApiError {
    fn from(err: PoolError) -> Self {
        EthApiError::PoolError(RpcPoolError::from(err))
    }
}

/// Errors returned from a sign request.
#[derive(Debug, thiserror::Error)]
pub enum SignError {
    /// Error occurred while trying to sign data.
    #[error("Could not sign")]
    CouldNotSign,
    /// Signer for the requested account is not found.
    #[error("Unknown account")]
    NoAccount,
    /// TypedData has an invalid format.
    #[error("Given typed data is not valid")]
    InvalidTypedData,
    /// Invalid transaction request in `sign_transaction`.
    #[error("Invalid transaction request")]
    InvalidTransactionRequest,
    /// No chain ID was given.
    #[error("No chain id")]
    NoChainId,
}

#[cfg(feature = "native")]
/// Converts the evm [ExecutionResult] into a result where `Ok` variant is the output bytes if it is
/// [ExecutionResult::Success].
pub(crate) fn ensure_success(
    result: revm::primitives::ExecutionResult,
) -> EthResult<reth_primitives::Bytes> {
    match result {
        revm::primitives::ExecutionResult::Success { output, .. } => Ok(output.into_data()),
        revm::primitives::ExecutionResult::Revert { output, .. } => {
            Err(RpcInvalidTransactionError::Revert(RevertError::new(output.into())).into())
        }
        revm::primitives::ExecutionResult::Halt { reason, gas_used } => {
            Err(RpcInvalidTransactionError::halt(reason, gas_used).into())
        }
    }
}

#[cfg(feature = "native")]
/// Constructs an invalid params JSON-RPC error.
pub(crate) fn invalid_params_rpc_err(
    msg: impl Into<String>,
) -> jsonrpsee::types::error::ErrorObject<'static> {
    rpc_err(jsonrpsee::types::error::INVALID_PARAMS_CODE, msg, None)
}

#[cfg(feature = "native")]
/// Constructs an internal JSON-RPC error.
fn internal_rpc_err(msg: impl Into<String>) -> jsonrpsee::types::error::ErrorObject<'static> {
    rpc_err(jsonrpsee::types::error::INTERNAL_ERROR_CODE, msg, None)
}

#[cfg(feature = "native")]
/// Constructs an internal JSON-RPC error with code and message
fn rpc_error_with_code(
    code: i32,
    msg: impl Into<String>,
) -> jsonrpsee::types::error::ErrorObject<'static> {
    rpc_err(code, msg, None)
}

#[cfg(feature = "native")]
/// Constructs a JSON-RPC error, consisting of `code`, `message` and optional `data`.
fn rpc_err(
    code: i32,
    msg: impl Into<String>,
    data: Option<&[u8]>,
) -> jsonrpsee::types::error::ErrorObject<'static> {
    jsonrpsee::types::error::ErrorObject::owned(
        code,
        msg.into(),
        data.map(|data| {
            jsonrpsee::core::to_json_raw_value(&format!("0x{}", hex::encode(data)))
                .expect("serializing String does fail")
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_out_error() {
        let err = EthApiError::ExecutionTimedOut(Duration::from_secs(10));
        assert_eq!(err.to_string(), "execution aborted (timeout = 10s)");
    }
}