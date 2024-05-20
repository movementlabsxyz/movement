use core::fmt::Display;
use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub Vec<u8>);

impl Id {
	pub fn test() -> Self {
		Self(vec![0])
	}

	pub fn to_vec(&self) -> Vec<u8> {
		self.0.clone()
	}

	pub fn genesis_block() -> Self {
		Self(vec![0])
	}
}

impl Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", &self.0)
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Transaction(pub Vec<u8>);

impl From<Vec<u8>> for Transaction {
	fn from(data: Vec<u8>) -> Self {
		Self(data)
	}
}

impl Transaction {
	pub fn new(data: Vec<u8>) -> Self {
		Self(data)
	}

	pub fn id(&self) -> Id {
		let mut hasher = sha2::Sha256::new();
		hasher.update(&self.0);
		Id(hasher.finalize().to_vec())
	}

	pub fn test() -> Self {
		Self(vec![0])
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
}

impl Block {
	pub fn new(metadata: BlockMetadata, parent: Vec<u8>, transactions: Vec<Transaction>) -> Self {
		Self { metadata, parent, transactions }
	}

	pub fn id(&self) -> Id {
		let mut hasher = sha2::Sha256::new();
		hasher.update(&self.parent);
		for transaction in &self.transactions {
			hasher.update(&transaction.0);
		}
		Id(hasher.finalize().to_vec())
	}

	pub fn test() -> Self {
		Self {
			metadata: BlockMetadata::BlockMetadata,
			parent: vec![0],
			transactions: vec![Transaction::test()],
		}
	}

	pub fn add_transaction(&mut self, transaction: Transaction) {
		self.transactions.push(transaction);
	}
}

