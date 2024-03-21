use std::ops::Range;

use aptos_crypto::{bls12381::Signature, hash::HashValue};
use aptos_sdk::types::account_address::AccountAddress;
use aptos_consensus_types::{block::Block, block_data::BlockData};
use reth_primitives::{Header, SealedHeader, TransactionSigned, TransactionSignedEcRecovered};
use revm::primitives::{Address, EVMError, B256};

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Clone)]
pub(crate) struct BlockEnv {
    /// This block's id as a hash value, it is generated at call time
    pub(crate) id: HashValue,
    /// The account for fees 
    pub(crate) coinbase: AccountAddress,
    /// The container for the actual block
    pub(crate) block_data: BlockData,
}

impl Default for BlockEnv {
    fn default() -> Self {
        Self {
            id: Default::default(),
            coinbase: Default::default(),
            block_data: Default::default(),
        }
    }
}

/// RLP encoded evm transaction.
#[derive(
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
    Debug,
    PartialEq,
    Clone,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct RlpEvmTransaction {
    /// Rlp data.
    pub rlp: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct TransactionSignedAndRecovered {
    /// Signer of the transaction
    pub(crate) signer: Address,
    /// Signed transaction
    pub(crate) signed_transaction: TransactionSigned,
    /// Block the transaction was added to
    pub(crate) block_number: u64,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SealedBlock {
    /// Block header.
    pub(crate) header: SealedHeader,

    /// Transactions in this block.
    pub(crate) transactions: Range<u64>,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Receipt {
    pub(crate) receipt: reth_primitives::Receipt,
    pub(crate) gas_used: u64,
    pub(crate) log_index_start: u64,
    pub(crate) error: Option<EVMError<u8>>,
}

impl From<TransactionSignedAndRecovered> for TransactionSignedEcRecovered {
    fn from(value: TransactionSignedAndRecovered) -> Self {
        TransactionSignedEcRecovered::from_signed_transaction(
            value.signed_transaction,
            value.signer,
        )
    }
}

pub(crate) enum BlockTransactions {
    Full(Vec<Block>),
    Signatures(Vec<Signature>) 
}

pub(crate) struct SovAptosBlock {
    pub(crate) block: Block,
    pub(crate) transactions: BlockTransactions,
}
