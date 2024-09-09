use crate::transaction::Transaction;
use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(
	Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct Id([u8; 32]);

impl Id {
	pub fn new(data: [u8; 32]) -> Self {
		Self(data)
	}

	pub fn inner(&self) -> &[u8; 32] {
		&self.0
	}

	pub fn test() -> Self {
		Self([0; 32])
	}

	pub fn to_vec(&self) -> Vec<u8> {
		self.0.into()
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
pub struct TransactionEntry {
	consumer_id: Id,
	data: Transaction,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AtomicTransactionBundle {
	sequencer_id: Id,
	transactions: Vec<TransactionEntry>,
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
