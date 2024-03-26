use aptos_api_types::{AccountData, MoveModule, MoveModuleBytecode, MoveResource};
use std::ops::Range;

use crate::aptos::db::SovAptosDb;
use crate::aptos::AccountInfo;
use aptos_consensus_types::{block::Block, block_data::BlockData};
use aptos_crypto::{bls12381::Signature, hash::HashValue};
use aptos_db::ledger_db::LedgerDb;
use aptos_sdk::rest_client::Account;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_storage_interface::state_view::DbStateView;
use aptos_types::on_chain_config::Version;
use auto_impl::auto_impl;
use reth_primitives::{Header, SealedHeader, TransactionSigned, TransactionSignedEcRecovered};
use reth_revm::precompile::HashMap;
use revm::primitives::{Address, EVMError, B256};

pub type SovLedgerDb = LedgerDb;

/// Aptos database interface
/// This trait is loosely modelled on `revm::Database` as this trait is used
/// in the sov-aptos module.
#[auto_impl(&mut, Box)]
pub trait AptosStorage {
	/// The database error type.
	type Error;

	/// Get basic account information.
	fn account(&mut self, account: Account) -> Result<AccountInfo, Self::Error>;

	/// Get Move resources for an account.
	fn resources(&mut self, account: Account) -> Result<Vec<MoveResource>, Self::Error>;

	/// Get modules for an account.
	fn modules(&mut self, account: Account) -> Result<Vec<MoveModule>, Self::Error>;
}

#[auto_impl(&mut, Box)]
pub trait AptosStorageCommit {
	/// Commit changes to the database.
	fn commit(&mut self, changes: HashMap<AccountAddress, Account>);
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Clone)]
pub(crate) struct BlockEnv {
	/// This block's id as a hash value, it is generated at call time
	pub(crate) id: HashValue,
	/// The account for fees
	pub(crate) coinbase: Address,
	/// The container for the actual block
	pub(crate) block_data: Option<BlockData>,
}

impl Default for BlockEnv {
	fn default() -> Self {
		Self { id: Default::default(), coinbase: Default::default(), block_data: None }
	}
}

/// RLP encoded Aptos transaction.
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
	Signatures(Vec<Signature>),
}

pub(crate) struct SovAptosBlock {
	pub(crate) block: Block,
	pub(crate) transactions: BlockTransactions,
}
