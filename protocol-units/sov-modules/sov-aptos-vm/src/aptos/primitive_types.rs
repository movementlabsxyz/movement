use aptos_api_types::{
	AccountData, Event, MoveModule, MoveModuleBytecode, MoveResource, Transaction,
};
use poem_openapi::__private::serde_json;
use std::ops::Range;

use crate::aptos::db::SovAptosDb;
use crate::aptos::AccountInfo;
use aptos_consensus_types::{block::Block, block_data::BlockData};
use aptos_crypto::{bls12381, bls12381::Signature, hash::HashValue};
use aptos_sdk::move_types::metadata::Metadata as AptosMetadata;
use aptos_sdk::rest_client::Account;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_storage_interface::state_view::DbStateView;
use aptos_types::on_chain_config::Version;
use aptos_types::state_store::state_key::StateKey;
use auto_impl::auto_impl;
use move_core_types::metadata::Metadata as MoveMetadata;
use reth_primitives::{Header, SealedHeader, TransactionSigned, TransactionSignedEcRecovered};
use reth_revm::precompile::HashMap;
use revm::primitives::{Address, EVMError, B256};
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

use aptos_types::state_store::state_value::StateValue;
use aptos_types::validator_signer::ValidatorSigner;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize};

#[cfg_attr(feature = "native", derive(serde::Serialize), derive(serde::Deserialize))]

pub struct StateValueWrapper(pub(crate) StateValue);

impl BorshSerialize for StateValueWrapper {
	fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
		writer.write_all(&serde_json::to_vec(&self.0)?)?;
		Ok(())
	}
}

impl BorshDeserialize for StateValueWrapper {
	fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
		Ok(Self(serde_json::from_slice(buf)?))
	}
	fn deserialize_reader<R>(_: &mut R) -> Result<Self, std::io::Error>
	where
		R: std::io::Read,
	{
		todo!()
	}
}

impl StateValueWrapper {
	pub fn new(state_key: StateValue) -> Self {
		Self(state_key)
	}
}

impl Into<StateValue> for StateValueWrapper {
	fn into(self) -> StateValue {
		self.0
	}
}

#[cfg_attr(feature = "native", derive(serde::Serialize), derive(serde::Deserialize))]
pub struct StateKeyWrapper(StateKey);

impl BorshSerialize for StateKeyWrapper {
	fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
		writer.write_all(&serde_json::to_vec(&self.0)?)?;
		Ok(())
	}
}

impl BorshDeserialize for StateKeyWrapper {
	fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
		Ok(Self(serde_json::from_slice(buf)?))
	}
	fn deserialize_reader<R>(_: &mut R) -> Result<Self, std::io::Error>
	where
		R: std::io::Read,
	{
		todo!()
	}
}

impl StateKeyWrapper {
	pub fn new(state_key: StateKey) -> Self {
		Self(state_key)
	}
}

impl Into<StateKey> for StateKeyWrapper {
	fn into(self) -> StateKey {
		self.0
	}
}

pub struct TransactionWrapper(Transaction);

impl BorshSerialize for TransactionWrapper {
	fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
		writer.write_all(&serde_json::to_vec(&self.0)?)?;
		Ok(())
	}
}

impl Into<Transaction> for TransactionWrapper {
	fn into(self) -> Transaction {
		self.0
	}
}

impl BorshDeserialize for TransactionWrapper {
	fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
		Ok(Self(serde_json::from_slice(buf)?))
	}
	fn deserialize_reader<R>(_: &mut R) -> Result<Self, std::io::Error>
	where
		R: std::io::Read,
	{
		todo!()
	}
}

#[derive(Debug, PartialEq, Clone)]
pub struct EventWrapper(Event);

impl BorshSerialize for EventWrapper {
	fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
		writer.write_all(&serde_json::to_vec(&self.0)?)?;
		Ok(())
	}
}

impl BorshDeserialize for EventWrapper {
	fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
		Ok(Self(serde_json::from_slice(buf)?))
	}
	fn deserialize_reader<R>(_: &mut R) -> Result<Self, std::io::Error>
	where
		R: std::io::Read,
	{
		todo!()
	}
}

pub struct MetadataWrapper(pub(crate) AptosMetadata);

impl From<MoveMetadata> for MetadataWrapper {
	fn from(metadata: MoveMetadata) -> Self {
		MetadataWrapper(AptosMetadata { key: metadata.key, value: metadata.value })
	}
}

pub struct ValidatorSignerWrapper(ValidatorSigner);

impl ValidatorSignerWrapper {
	pub fn new(signer: ValidatorSigner) -> Self {
		ValidatorSignerWrapper(signer)
	}
}

impl Serialize for ValidatorSignerWrapper {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let mut map = serializer.serialize_map(Some(2))?;
		map.serialize_entry("author", &self.0.author())?;
		map.serialize_entry("private_key", &self.0.private_key().to_bytes())?;
		map.end()
	}
}
