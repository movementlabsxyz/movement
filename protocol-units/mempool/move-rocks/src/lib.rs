use mempool_util::{
	MempoolBlockOperations, MempoolBlockOperationsError, MempoolBlockOperationsResult,
	MempoolTransaction, MempoolTransactionOperations, MempoolTransactionOperationsError,
	MempoolTransactionOperationsResult,
};
use movement_types::{Block, Id};
use rocksdb::{BoundColumnFamily, ColumnFamilyDescriptor, Options, DB};
use serde_json;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum RocksdbMempoolError {
	#[error("Underlying db error")]
	RocksDbError(#[from] rocksdb::Error),
	#[error("Family handle missing")]
	FamilyHandleMissing,
}

pub type RocksdbMempoolResult<T> = Result<T, RocksdbMempoolError>;

trait DBExt {
	fn get_from_handle(
		&self,
		handle: &str,
		key: impl AsRef<[u8]>,
	) -> RocksdbMempoolResult<Option<Vec<u8>>>;

	fn put_to_handle(
		&self,
		handle: &str,
		key: impl AsRef<[u8]>,
		value: impl AsRef<[u8]>,
	) -> RocksdbMempoolResult<()>;

	fn delete_from_handle(&self, handle: &str, key: impl AsRef<[u8]>) -> RocksdbMempoolResult<()>;

	fn iter_from_handle(
		&self,
		handle: &str,
	) -> RocksdbMempoolResult<(Arc<BoundColumnFamily>, rocksdb::DBIterator)>;
}

impl DBExt for rocksdb::DB {
	fn get_from_handle(
		&self,
		handle: &str,
		key: impl AsRef<[u8]>,
	) -> RocksdbMempoolResult<Option<Vec<u8>>> {
		let cf_handle = self.cf_handle(handle).ok_or(RocksdbMempoolError::FamilyHandleMissing)?;
		self.get_cf(&cf_handle, key).map_err(RocksdbMempoolError::from)
	}

	fn put_to_handle(
		&self,
		handle: &str,
		key: impl AsRef<[u8]>,
		value: impl AsRef<[u8]>,
	) -> RocksdbMempoolResult<()> {
		let cf_handle = self.cf_handle(handle).ok_or(RocksdbMempoolError::FamilyHandleMissing)?;
		self.put_cf(&cf_handle, key, value).map_err(RocksdbMempoolError::from)
	}

	fn delete_from_handle(&self, handle: &str, key: impl AsRef<[u8]>) -> RocksdbMempoolResult<()> {
		let cf_handle = self.cf_handle(handle).ok_or(RocksdbMempoolError::FamilyHandleMissing)?;
		self.delete_cf(&cf_handle, key).map_err(RocksdbMempoolError::from)
	}

	fn iter_from_handle(
		&self,
		handle: &str,
	) -> RocksdbMempoolResult<(Arc<BoundColumnFamily>, rocksdb::DBIterator)> {
		let cf_handle = self.cf_handle(handle).ok_or(RocksdbMempoolError::FamilyHandleMissing)?;
		let iterator = self.iterator_cf(&cf_handle, rocksdb::IteratorMode::Start);
		Ok((cf_handle, iterator))
	}
}

#[derive(Debug, Clone)]
pub struct RocksdbMempool {
	db: Arc<RwLock<DB>>,
}
impl RocksdbMempool {
	pub fn try_new(path: &str) -> RocksdbMempoolResult<Self> {
		let mut options = Options::default();
		options.create_if_missing(true);
		options.create_missing_column_families(true);

		let mempool_transactions_cf =
			ColumnFamilyDescriptor::new("mempool_transactions", Options::default());
		let transaction_truths_cf =
			ColumnFamilyDescriptor::new("transaction_truths", Options::default());
		let blocks_cf = ColumnFamilyDescriptor::new("blocks", Options::default());
		let transaction_lookups_cf =
			ColumnFamilyDescriptor::new("transaction_lookups", Options::default());

		let db = DB::open_cf_descriptors(
			&options,
			path,
			vec![mempool_transactions_cf, transaction_truths_cf, blocks_cf, transaction_lookups_cf],
		)?;

		Ok(RocksdbMempool { db: Arc::new(RwLock::new(db)) })
	}

	pub fn construct_mempool_transaction_key(transaction: &MempoolTransaction) -> String {
		// pad to 32 characters
		let slot_seconds_str = format!("{:032}", transaction.timestamp);

		// Assuming transaction.transaction.id() returns a hex string of length 32
		let transaction_id_hex = transaction.transaction.id(); // This should be a String of hex characters

		// Concatenate the two parts to form a 48-character hex string key
		let key = format!("{}:{}", slot_seconds_str, transaction_id_hex);

		key
	}

	/// Helper function to retrieve the key for mempool transaction from the lookup table.
	async fn get_mempool_transaction_key(
		&self,
		transaction_id: &Id,
	) -> RocksdbMempoolResult<Option<Vec<u8>>> {
		let db = self.db.read().await;
		db.get_from_handle("transaction_lookups", transaction_id.to_vec())
	}
}

#[async_trait::async_trait]
impl MempoolTransactionOperations for RocksdbMempool {
	type Error = RocksdbMempoolError;

	async fn has_mempool_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<bool, RocksdbMempoolError> {
		let key = self.get_mempool_transaction_key(&transaction_id).await?;
		match key {
			Some(k) => {
				let db = self.db.read().await;
				db.get_from_handle("mempool_transactions", k)
					.map(|v| v.is_some())
					.map_err(From::from)
			},
			None => Ok(false),
		}
	}

	async fn add_mempool_transaction(
		&self,
		tx: MempoolTransaction,
	) -> MempoolTransactionOperationsResult<(), RocksdbMempoolError> {
		let serialized_tx = serde_json::to_vec(&tx)
			.map_err(|e| MempoolTransactionOperationsError::SerializationError(e.to_string()))?;

		let db = self.db.write().await;

		let key = Self::construct_mempool_transaction_key(&tx);
		db.put_to_handle("mempool_transactions", &key, &serialized_tx)?;
		db.put_to_handle("transaction_lookups", tx.transaction.id().to_vec(), &key)?;

		Ok(())
	}

	async fn remove_mempool_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<(), RocksdbMempoolError> {
		let key = self.get_mempool_transaction_key(&transaction_id).await?;

		match key {
			Some(k) => {
				let db = self.db.write().await;
				db.delete_from_handle("mempool_transactions", k)?;
				db.delete_from_handle("transaction_lookups", transaction_id.to_vec())?;
			},
			None => (),
		}
		Ok(())
	}

	// Updated method signatures and implementations go here
	async fn get_mempool_transaction(
		&self,
		transaction_id: Id,
	) -> MempoolTransactionOperationsResult<Option<MempoolTransaction>, RocksdbMempoolError> {
		let key = match self.get_mempool_transaction_key(&transaction_id).await? {
			Some(k) => k,
			None => return Ok(None), // If no key found in lookup, return None
		};

		let db = self.db.read().await;
		match db.get_from_handle("mempool_transactions", &key)? {
			Some(serialized_tx) => {
				let tx: MempoolTransaction =
					serde_json::from_slice(&serialized_tx).map_err(|e| {
						MempoolTransactionOperationsError::SerializationError(e.to_string())
					})?;
				Ok(Some(tx))
			},
			None => Ok(None),
		}
	}

	async fn pop_mempool_transaction(
		&self,
	) -> MempoolTransactionOperationsResult<Option<MempoolTransaction>, RocksdbMempoolError> {
		let db = self.db.write().await;

		let (_, mut iter) = db.iter_from_handle("mempool_transactions")?;

		match iter.next() {
			None => return Ok(None), // No transactions to pop
			Some(res) => {
				let (key, value) = res.map_err(RocksdbMempoolError::from)?;
				let tx: MempoolTransaction = serde_json::from_slice(&value).map_err(|e| {
					MempoolTransactionOperationsError::DeserializationError(e.to_string())
				})?;

				db.delete_from_handle("mempool_transactions", &key)?;

				db.delete_from_handle("transaction_lookups", tx.transaction.id().to_vec())?;

				Ok(Some(tx))
			},
		}
	}
}

#[async_trait::async_trait]
impl MempoolBlockOperations for RocksdbMempool {
	type Error = RocksdbMempoolError;

	async fn has_block(
		&self,
		block_id: Id,
	) -> MempoolBlockOperationsResult<bool, RocksdbMempoolError> {
		let db = self.db.read().await;
		Ok(db.get_from_handle("blocks", block_id.to_vec())?.is_some())
	}

	async fn add_block(
		&self,
		block: Block,
	) -> MempoolBlockOperationsResult<(), RocksdbMempoolError> {
		let serialized_block = serde_json::to_vec(&block)
			.map_err(|e| MempoolBlockOperationsError::SerializeError(e.to_string()))?;

		let db = self.db.write().await;
		db.put_to_handle("blocks", block.id().to_vec(), &serialized_block)?;
		Ok(())
	}

	async fn remove_block(
		&self,
		block_id: Id,
	) -> MempoolBlockOperationsResult<(), RocksdbMempoolError> {
		let db = self.db.write().await;
		db.delete_from_handle("blocks", block_id.to_vec())?;
		Ok(())
	}

	async fn get_block(
		&self,
		block_id: Id,
	) -> MempoolBlockOperationsResult<Option<Block>, RocksdbMempoolError> {
		let db = self.db.read().await;
		let serialized_block = db.get_from_handle("blocks", block_id.to_vec())?;
		match serialized_block {
			Some(serialized_block) => {
				let block: Block = serde_json::from_slice(&serialized_block).map_err(|e| {
					MempoolBlockOperationsError::DeserializationError(e.to_string())
				})?;
				Ok(Some(block))
			},
			None => Ok(None),
		}
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_types::Transaction;
	use tempfile::tempdir;

	use anyhow::Result;

	#[tokio::test]
	async fn test_rocksdb_mempool_basic_operations() -> Result<()> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let tx = MempoolTransaction::test();
		let tx_id = tx.id();
		mempool.add_mempool_transaction(tx.clone()).await?;
		assert!(mempool.has_mempool_transaction(tx_id.clone()).await?);
		let tx2 = mempool.get_mempool_transaction(tx_id.clone()).await?;
		assert_eq!(Some(tx), tx2);
		mempool.remove_mempool_transaction(tx_id.clone()).await?;
		assert!(!mempool.has_mempool_transaction(tx_id.clone()).await?);

		let block = Block::test();
		let block_id = block.id();
		mempool.add_block(block.clone()).await?;
		assert!(mempool.has_block(block_id.clone()).await?);
		let block2 = mempool.get_block(block_id.clone()).await?;
		assert_eq!(Some(block), block2);
		mempool.remove_block(block_id.clone()).await?;
		assert!(!mempool.has_block(block_id.clone()).await?);

		Ok(())
	}

	#[tokio::test]
	async fn test_rocksdb_transaction_operations() -> Result<()> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let tx = Transaction::test();
		let tx_id = tx.id();
		mempool.add_transaction(tx.clone()).await?;
		assert!(mempool.has_transaction(tx_id.clone()).await?);
		let tx2 = mempool.get_transaction(tx_id.clone()).await?;
		assert_eq!(Some(tx), tx2);
		mempool.remove_transaction(tx_id.clone()).await?;
		assert!(!mempool.has_transaction(tx_id.clone()).await?);

		Ok(())
	}

	#[tokio::test]
	async fn test_transaction_slot_based_ordering() -> Result<()> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let tx1 = MempoolTransaction::at_time(Transaction::new(vec![1]), 2);
		let tx2 = MempoolTransaction::at_time(Transaction::new(vec![2]), 64);
		let tx3 = MempoolTransaction::at_time(Transaction::new(vec![3]), 128);

		mempool.add_mempool_transaction(tx2.clone()).await?;
		mempool.add_mempool_transaction(tx1.clone()).await?;
		mempool.add_mempool_transaction(tx3.clone()).await?;

		let txs = mempool.pop_mempool_transactions(3).await?;
		assert_eq!(txs[0], tx1);
		assert_eq!(txs[1], tx2);
		assert_eq!(txs[2], tx3);

		Ok(())
	}
}
