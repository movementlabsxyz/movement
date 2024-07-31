pub mod algs;
use aptos_types::state_proof::StateProof;
use serde::{Deserialize, Serialize};
use core::fmt;

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub [u8; 32]);

impl Id {
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
pub struct Transaction {
	pub data: Vec<u8>,
	pub sequence_number: u64,
	pub id : Id,
}

impl Transaction {
	pub fn new(data: Vec<u8>, sequence_number: u64) -> Self {
		let mut hasher = blake3::Hasher::new();
		hasher.update(&data);
		hasher.update(&sequence_number.to_le_bytes());
		let id = Id(hasher.finalize().into());
		Self { data, sequence_number, id }
	}

	pub fn id(&self) -> Id {
		self.id.clone()
	}

	pub fn test() -> Self {
		Self::new(vec![0], 0)
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TransactionEntry {
	pub consumer_id: Id,
	pub data: Transaction,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AtomicTransactionBundle {
	pub sequencer_id: Id,
	pub transactions: Vec<TransactionEntry>,
}

impl TryFrom<AtomicTransactionBundle> for Transaction {
	type Error = anyhow::Error;

	fn try_from(value: AtomicTransactionBundle) -> Result<Self, Self::Error> {
		if value.transactions.len() == 1 {
			Ok(value.transactions[0].data.clone())
		} else {
			Err(anyhow::anyhow!("AtomicTransactionBundle must contain exactly one transaction"))
		}
	}
}

impl From<Transaction> for AtomicTransactionBundle {
	fn from(transaction: Transaction) -> Self {
		Self {
			sequencer_id: Id::default(),
			transactions: vec![TransactionEntry { consumer_id: Id::default(), data: transaction }],
		}
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockMetadata {
	#[default]
	BlockMetadata,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Block {
	pub metadata: BlockMetadata,
	pub parent: Vec<u8>,
	pub transactions: Vec<Transaction>,
	pub id : Id,
}

impl Block {
	pub fn new(metadata: BlockMetadata, parent: Vec<u8>, transactions: Vec<Transaction>) -> Self {

		let mut hasher = blake3::Hasher::new();
		hasher.update(&parent);
		for transaction in &transactions {
			hasher.update(&transaction.id().as_ref());
		}
		let id = Id(hasher.finalize().into());

		Self { metadata, parent, transactions, id }
	}

	pub fn id(&self) -> Id {
		self.id.clone()
	}

	pub fn test() -> Self {
		Self::new(
			BlockMetadata::BlockMetadata,
			vec![0],
			vec![Transaction::test()],
		)
	}

	pub fn add_transaction(&mut self, transaction: Transaction) {
		self.transactions.push(transaction);
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Commitment(pub [u8; 32]);

impl Commitment {
	pub fn test() -> Self {
		Self([0; 32])
	}

	/// Creates a commitment by making a cryptographic digest of the state proof.
	pub fn digest_state_proof(state_proof: &StateProof) -> Self {
		let mut hasher = blake3::Hasher::new();
		bcs::serialize_into(&mut hasher, &state_proof).expect("unexpected serialization error");
		Self(hasher.finalize().into())
	}
}

impl TryFrom<Vec<u8>> for Commitment {
	type Error = std::array::TryFromSliceError;

	fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(Self(data[..32].try_into()?))
	}
}

impl From<[u8; 32]> for Commitment {
	fn from(data: [u8; 32]) -> Self {
		Self(data)
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
	pub height: u64,
	pub block_id: Id,
	pub commitment: Commitment,
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
