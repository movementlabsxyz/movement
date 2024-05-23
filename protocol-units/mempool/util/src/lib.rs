use serde::{Deserialize, Serialize};

use movement_types::{Block, Id, Transaction};
use std::cmp::Ordering;
use thiserror::Error;

pub type BoxedStoreError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum MempoolTransactionOperationsError {
	#[error("Serialization error: {0}")]
	SerializationError(String),
	#[error("Deserialization error: {0}")]
	DeserializationError(String),
	#[error("Underlying store error")]
	StoreError(#[from] BoxedStoreError),
	#[error("Other error: {0}")]
	Other(String),
}

impl MempoolTransactionOperationsError {
	pub fn store_error<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
		Self::StoreError(Box::new(error))
	}
}

pub type MempoolTransactionOperationsResult<T> = Result<T, MempoolTransactionOperationsError>;

#[allow(async_fn_in_trait)]
pub trait MempoolTransactionOperations {
	// todo: move mempool_transaction methods into separate trait

	/// Checks whether a mempool transaction exists in the mempool.
	async fn has_mempool_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<bool>;

	/// Adds a mempool transaction to the mempool.
	async fn add_mempool_transaction(
		&self,
		tx: MempoolTransaction,
	) -> MempoolTransactionOperationsResult<()>;

	/// Removes a mempool transaction from the mempool.
	async fn remove_mempool_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<()>;

	/// Pops mempool transaction from the mempool.
	async fn pop_mempool_transaction(
		&self,
	) -> MempoolTransactionOperationsResult<Option<MempoolTransaction>>;

	/// Gets a mempool transaction from the mempool.
	async fn get_mempool_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<Option<MempoolTransaction>>;

	/// Pops the next n mempool transactions from the mempool.
	async fn pop_mempool_transactions(
		&self,
		n: usize,
	) -> MempoolTransactionOperationsResult<Vec<MempoolTransaction>> {
		let mut mempool_transactions = Vec::with_capacity(n);
		for _ in 0..n {
			if let Some(mempool_transaction) = self.pop_mempool_transaction().await? {
				mempool_transactions.push(mempool_transaction);
			} else {
				break;
			}
		}
		Ok(mempool_transactions)
	}

	/// Checks whether the mempool has the transaction.
	async fn has_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<bool> {
		self.has_mempool_transaction(transaction_id).await
	}

	/// Adds a transaction to the mempool.
	async fn add_transaction(&self, tx: Transaction) -> MempoolTransactionOperationsResult<()> {
		if self.has_transaction(tx.id()).await? {
			return Ok(());
		}

		let mempool_transaction = MempoolTransaction::slot_now(tx);
		self.add_mempool_transaction(mempool_transaction).await
	}

	/// Removes a transaction from the mempool.
	async fn remove_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<()> {
		self.remove_mempool_transaction(transaction_id).await
	}

	/// Pops transaction from the mempool.
	async fn pop_transaction(&self) -> MempoolTransactionOperationsResult<Option<Transaction>> {
		let mempool_transaction = self.pop_mempool_transaction().await?;
		Ok(mempool_transaction.map(|mempool_transaction| mempool_transaction.transaction))
	}

	/// Gets a transaction from the mempool.
	async fn get_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<Option<Transaction>> {
		let mempool_transaction = self.get_mempool_transaction(transaction_id).await?;
		Ok(mempool_transaction.map(|mempool_transaction| mempool_transaction.transaction))
	}

	/// Pops the next n transactions from the mempool.
	async fn pop_transactions(
		&self,
		n: usize,
	) -> MempoolTransactionOperationsResult<Vec<Transaction>> {
		let mempool_transactions = self.pop_mempool_transactions(n).await?;
		Ok(mempool_transactions
			.into_iter()
			.map(|mempool_transaction| mempool_transaction.transaction)
			.collect())
	}
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum MempoolBlockOperationsError {
	#[error("Store error: {0}")]
	StoreError(#[from] BoxedStoreError),
	#[error("Serialization error: {0}")]
	SerializeError(String),
	#[error("Deserialization error: {0}")]
	DeserializationError(String),
}

impl MempoolBlockOperationsError {
	pub fn store_error<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
		Self::StoreError(Box::new(error))
	}
}

pub type MempoolBlockOperationsResult<T> = Result<T, MempoolBlockOperationsError>;

#[allow(async_fn_in_trait)]
pub trait MempoolBlockOperations {
	/// Checks whether a block exists in the mempool.
	async fn has_block(&self, block_id: Id) -> MempoolBlockOperationsResult<bool>;

	/// Adds a block to the mempool.
	async fn add_block(&self, block: Block) -> MempoolBlockOperationsResult<()>;

	/// Removes a block from the mempool.
	async fn remove_block(&self, block_id: Id) -> MempoolBlockOperationsResult<()>;

	/// Gets a block from the mempool.
	async fn get_block(&self, block_id: Id) -> MempoolBlockOperationsResult<Option<Block>>;
}

/// Wraps a transaction with a timestamp for help ordering.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MempoolTransaction {
	pub transaction: Transaction,
	pub timestamp: u64,
	pub slot_seconds: u64,
}

impl PartialOrd for MempoolTransaction {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

/// Ordered first by slot_seconds, then by transaction.
/// This allows us to use a BTreeSet to order transactions by slot_seconds, and then by transaction and pop them off in order.
impl Ord for MempoolTransaction {
	fn cmp(&self, other: &Self) -> Ordering {
		// First, compare by slot_seconds
		match self.slot_seconds.cmp(&other.slot_seconds) {
			Ordering::Equal => {},
			non_equal => return non_equal,
		}
		// If slot_seconds are equal, then compare by transaction
		self.transaction.cmp(&other.transaction)
	}
}

impl MempoolTransaction {
	const SLOT_SECONDS: u64 = 2;

	/// Creates a test MempoolTransaction.
	pub fn test() -> Self {
		Self { transaction: Transaction::test(), timestamp: 0, slot_seconds: Self::SLOT_SECONDS }
	}

	pub fn at_time(transaction: Transaction, timestamp: u64) -> Self {
		let floor = (timestamp / Self::SLOT_SECONDS) * Self::SLOT_SECONDS;
		Self { transaction, timestamp: floor, slot_seconds: Self::SLOT_SECONDS }
	}

	pub fn new(transaction: Transaction, timestamp: u64, slot_seconds: u64) -> Self {
		Self { transaction, timestamp, slot_seconds }
	}

	/// Creates a new MempoolTransaction with the current timestamp floored to the nearest slot.
	/// todo: probably want to move this out to a factory.
	pub fn slot_now(transaction: Transaction) -> MempoolTransaction {
		let timestamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs();

		Self::at_time(transaction, timestamp)
	}

	pub fn id(&self) -> Id {
		self.transaction.id()
	}
}
