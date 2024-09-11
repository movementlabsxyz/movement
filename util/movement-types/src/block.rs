use crate::transaction::Transaction;
use aptos_types::state_proof::StateProof;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::collections::btree_set;
use std::collections::BTreeSet;

pub type Transactions<'a> = btree_set::Iter<'a, Transaction>;

#[derive(
	Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct Id([u8; 32]);

impl Id {
	pub fn new(data: [u8; 32]) -> Self {
		Self(data)
	}

	pub fn as_bytes(&self) -> &[u8; 32] {
		&self.0
	}

	pub fn test() -> Self {
		Self([0; 32])
	}

	pub fn to_vec(&self) -> Vec<u8> {
		self.0.into()
	}

	pub fn genesis_block() -> Self {
		Self([0; 32])
	}
}

impl AsRef<[u8]> for Id {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl fmt::Display for Id {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for byte in &self.0 {
			write!(f, "{:02x}", byte)?;
		}
		Ok(())
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockMetadata {
	#[default]
	BlockMetadata,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Block {
	metadata: BlockMetadata,
	parent: Id,
	transactions: BTreeSet<Transaction>,
	id: Id,
}

impl Block {
	pub fn new(metadata: BlockMetadata, parent: Id, transactions: BTreeSet<Transaction>) -> Self {
		let mut hasher = blake3::Hasher::new();
		hasher.update(parent.as_bytes());
		for transaction in &transactions {
			hasher.update(&transaction.id().as_ref());
		}
		let id = Id(hasher.finalize().into());

		Self { metadata, parent, transactions, id }
	}

	pub fn into_parts(self) -> (BlockMetadata, Id, BTreeSet<Transaction>, Id) {
		(self.metadata, self.parent, self.transactions, self.id)
	}

	pub fn id(&self) -> Id {
		self.id
	}

	pub fn parent(&self) -> Id {
		self.parent
	}

	pub fn transactions(&self) -> Transactions {
		self.transactions.iter()
	}

	pub fn metadata(&self) -> &BlockMetadata {
		&self.metadata
	}

	pub fn test() -> Self {
		Self::new(
			BlockMetadata::BlockMetadata,
			Id::test(),
			BTreeSet::from_iter(vec![Transaction::test()]),
		)
	}

	pub fn add_transaction(&mut self, transaction: Transaction) {
		self.transactions.insert(transaction);
	}
}

#[derive(
	Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct Commitment([u8; 32]);

impl Commitment {
	pub fn new(data: [u8; 32]) -> Self {
		Self(data)
	}

	pub fn test() -> Self {
		Self([0; 32])
	}

	pub fn as_bytes(&self) -> &[u8; 32] {
		&self.0
	}

	/// Creates a commitment by making a cryptographic digest of the state proof.
	pub fn digest_state_proof(state_proof: &StateProof) -> Self {
		let mut hasher = blake3::Hasher::new();
		bcs::serialize_into(&mut hasher, &state_proof).expect("unexpected serialization error");
		Self(hasher.finalize().into())
	}
}

impl From<Commitment> for [u8; 32] {
	fn from(commitment: Commitment) -> [u8; 32] {
		commitment.0
	}
}

impl From<Commitment> for Vec<u8> {
	fn from(commitment: Commitment) -> Vec<u8> {
		commitment.0.into()
	}
}

impl fmt::Display for Commitment {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for byte in &self.0 {
			write!(f, "{:02x}", byte)?;
		}
		Ok(())
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockCommitment {
	height: u64,
	block_id: Id,
	commitment: Commitment,
}

impl BlockCommitment {
	pub fn new(height: u64, block_id: Id, commitment: Commitment) -> Self {
		Self { height, block_id, commitment }
	}

	pub fn height(&self) -> u64 {
		self.height
	}

	pub fn block_id(&self) -> &Id {
		&self.block_id
	}

	pub fn commitment(&self) -> Commitment {
		self.commitment
	}

	pub fn test() -> Self {
		Self::new(0, Id::test(), Commitment::test())
	}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockCommitmentRejectionReason {
	InvalidBlockId,
	InvalidCommitment,
	InvalidHeight,
	ContractError,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockCommitmentEvent {
	Accepted(BlockCommitment),
	Rejected { height: u64, reason: BlockCommitmentRejectionReason },
}
