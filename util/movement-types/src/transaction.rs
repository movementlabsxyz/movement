use core::fmt;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

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

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct Transaction {
	data: Vec<u8>,
	// Application priority is stored low to high, i.e., 0 is the highest priority.
	application_priority: u64,
	sequence_number: u64,
	id: Id,
}

impl Transaction {
	pub fn new(data: Vec<u8>, application_priority: u64, sequence_number: u64) -> Self {
		let mut hasher = blake3::Hasher::new();
		hasher.update(&data);
		hasher.update(&sequence_number.to_le_bytes());
		let id = Id(hasher.finalize().into());
		Self { data, sequence_number, application_priority, id }
	}

	pub fn id(&self) -> Id {
		self.id
	}

	pub fn data(&self) -> &[u8] {
		&self.data
	}

	/// Returns the application priority of the transaction.
	/// The lower the value, the higher the priority.
	/// If you are using a high value, high priority scheme, simply subtract the priority from the maximum value.
	pub fn application_priority(&self) -> u64 {
		self.application_priority
	}

	pub fn sequence_number(&self) -> u64 {
		self.sequence_number
	}

	pub fn test() -> Self {
		Self::new(vec![0], 0, 0)
	}
}

impl Ord for Transaction {
	fn cmp(&self, other: &Self) -> Ordering {
		// First, compare by application_priority
		match self.application_priority.cmp(&other.application_priority) {
			Ordering::Equal => {}
			non_equal => return non_equal,
		}

		// Then compare by sequence number
		match self.sequence_number().cmp(&other.sequence_number()) {
			Ordering::Equal => {}
			non_equal => return non_equal,
		}

		// If sequence number is equal, then compare by transaction on the whole
		self.id().cmp(&other.id())
	}
}

impl PartialOrd for Transaction {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_transaction_ordering() {
		// priority based ordering
		let transaction = Transaction::new(vec![1], 0, 0);
		let transaction2 = Transaction::new(vec![1], 1, 0);
		let transaction3 = Transaction::new(vec![1], 2, 0);

		assert!(transaction < transaction2);
		assert!(transaction2 < transaction3);

		// sequencer number based ordering
		let transaction = Transaction::test();
		let transaction2 = Transaction::new(vec![1], 0, 1);
		let transaction3 = Transaction::new(vec![1], 0, 2);

		assert!(transaction < transaction2);
		assert!(transaction2 < transaction3);
	}
}
