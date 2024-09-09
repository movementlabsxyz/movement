use anyhow::Error;
use bcs;
use mempool_util::{MempoolBlockOperations, MempoolTransaction, MempoolTransactionOperations};
use movement_types::{
	block::{self, Block},
	transaction,
};
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::fmt::Write;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RocksdbMempool {
	db: Arc<DB>,
}
impl RocksdbMempool {
	pub fn try_new(path: &str) -> Result<Self, Error> {
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
		)
		.map_err(|e| Error::new(e))?;

		Ok(RocksdbMempool { db: Arc::new(db) })
	}

	pub fn construct_mempool_transaction_key(transaction: &MempoolTransaction) -> String {
		// Pre-allocate a string with the required capacity
		let mut key = String::with_capacity(32 + 1 + 32 + 1 + 32);
		// Write key components. The numbers are zero-padded to 32 characters.
		key.write_fmt(format_args!(
			"{:032}:{:032}:{}",
			transaction.timestamp,
			transaction.transaction.sequence_number(),
			transaction.transaction.id(),
		))
		.unwrap();
		key
	}

	fn internal_get_mempool_transaction_key(
		db: Arc<DB>,
		transaction_id: transaction::Id,
	) -> Result<Option<Vec<u8>>, Error> {
		let cf_handle = db
			.cf_handle("transaction_lookups")
			.ok_or_else(|| Error::msg("CF handle not found"))?;
		db.get_cf(&cf_handle, transaction_id.to_vec()).map_err(|e| Error::new(e))
	}

	/// Helper function to retrieve the key for mempool transaction from the lookup table.
	async fn get_mempool_transaction_key(
		&self,
		transaction_id: transaction::Id,
	) -> Result<Option<Vec<u8>>, Error> {
		let db = self.db.clone();
		let transaction_id = transaction_id.clone();
		tokio::task::spawn_blocking(move || {
			Self::internal_get_mempool_transaction_key(db, transaction_id)
		})
		.await?
	}

	fn internal_has_mempool_transaction(
		db: Arc<DB>,
		transaction_id: transaction::Id,
	) -> Result<bool, Error> {
		let key = Self::internal_get_mempool_transaction_key(db.clone(), transaction_id)?;
		match key {
			Some(k) => {
				let cf_handle = db
					.cf_handle("mempool_transactions")
					.ok_or_else(|| Error::msg("CF handle not found"))?;
				Ok(db.get_cf(&cf_handle, k)?.is_some())
			}
			None => Ok(false),
		}
	}
}

impl MempoolTransactionOperations for RocksdbMempool {
	async fn has_mempool_transaction(
		&self,
		transaction_id: transaction::Id,
	) -> Result<bool, Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			Self::internal_has_mempool_transaction(db, transaction_id)
		})
		.await?
	}

	async fn add_mempool_transactions(
		&self,
		transactions: Vec<MempoolTransaction>,
	) -> Result<(), anyhow::Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let mempool_transactions_cf_handle = db
				.cf_handle("mempool_transactions")
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let transaction_lookups_cf_handle = db
				.cf_handle("transaction_lookups")
				.ok_or_else(|| Error::msg("CF handle not found"))?;

			for transaction in transactions {
				if Self::internal_has_mempool_transaction(db.clone(), transaction.transaction.id())?
				{
					continue;
				}

				let serialized_transaction = bcs::to_bytes(&transaction)?;
				let key = Self::construct_mempool_transaction_key(&transaction);
				db.put_cf(&mempool_transactions_cf_handle, &key, &serialized_transaction)?;
				db.put_cf(
					&transaction_lookups_cf_handle,
					transaction.transaction.id().to_vec(),
					&key,
				)?;
			}
			Ok::<(), Error>(())
		})
		.await??;
		Ok(())
	}

	async fn add_mempool_transaction(&self, transaction: MempoolTransaction) -> Result<(), Error> {
		let serialized_transaction = bcs::to_bytes(&transaction)?;
		let db = self.db.clone();

		tokio::task::spawn_blocking(move || {
			let mempool_transactions_cf_handle = db
				.cf_handle("mempool_transactions")
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let transaction_lookups_cf_handle = db
				.cf_handle("transaction_lookups")
				.ok_or_else(|| Error::msg("CF handle not found"))?;

			let key = Self::construct_mempool_transaction_key(&transaction);
			db.put_cf(&mempool_transactions_cf_handle, &key, &serialized_transaction)?;
			db.put_cf(&transaction_lookups_cf_handle, transaction.transaction.id().to_vec(), &key)?;
			Ok::<(), Error>(())
		})
		.await??;

		Ok(())
	}

	async fn remove_mempool_transaction(
		&self,
		transaction_id: transaction::Id,
	) -> Result<(), Error> {
		let key = self.get_mempool_transaction_key(transaction_id).await?;
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			match key {
				Some(k) => {
					let cf_handle = db
						.cf_handle("mempool_transactions")
						.ok_or_else(|| Error::msg("CF handle not found"))?;
					db.delete_cf(&cf_handle, k)?;
					let lookups_cf_handle = db
						.cf_handle("transaction_lookups")
						.ok_or_else(|| Error::msg("CF handle not found"))?;
					db.delete_cf(&lookups_cf_handle, transaction_id.to_vec())?;
				}
				None => (),
			}
			Ok::<(), Error>(())
		})
		.await??;
		Ok(())
	}

	// Updated method signatures and implementations go here
	async fn get_mempool_transaction(
		&self,
		transaction_id: transaction::Id,
	) -> Result<Option<MempoolTransaction>, Error> {
		let key = match self.get_mempool_transaction_key(transaction_id).await? {
			Some(k) => k,
			None => return Ok(None), // If no key found in lookup, return None
		};
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle = db
				.cf_handle("mempool_transactions")
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			match db.get_cf(&cf_handle, &key)? {
				Some(serialized_transaction) => {
					let transaction: MempoolTransaction = bcs::from_bytes(&serialized_transaction)?;
					Ok(Some(transaction))
				}
				None => Ok(None),
			}
		})
		.await?
	}

	async fn pop_mempool_transaction(&self) -> Result<Option<MempoolTransaction>, Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle = db
				.cf_handle("mempool_transactions")
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let mut iter = db.iterator_cf(&cf_handle, rocksdb::IteratorMode::Start);

			match iter.next() {
				None => return Ok(None), // No transactions to pop
				Some(res) => {
					let (key, value) = res?;
					let transaction: MempoolTransaction = bcs::from_bytes(&value)?;
					db.delete_cf(&cf_handle, &key)?;

					// Optionally, remove from the lookup table as well
					let lookups_cf_handle = db
						.cf_handle("transaction_lookups")
						.ok_or_else(|| Error::msg("CF handle not found"))?;
					db.delete_cf(&lookups_cf_handle, transaction.transaction.id().to_vec())?;

					Ok(Some(transaction))
				}
			}
		})
		.await?
	}

	async fn pop_mempool_transactions(
		&self,
		n: usize,
	) -> Result<Vec<MempoolTransaction>, anyhow::Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle = db
				.cf_handle("mempool_transactions")
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let mut iter = db.iterator_cf(&cf_handle, rocksdb::IteratorMode::Start);

			let mut mempool_transactions = Vec::with_capacity(n as usize);
			while let Some(res) = iter.next() {
				let (key, value) = res?;
				let transaction: MempoolTransaction = bcs::from_bytes(&value)?;
				db.delete_cf(&cf_handle, &key)?;

				// Optionally, remove from the lookup table as well
				let lookups_cf_handle = db
					.cf_handle("transaction_lookups")
					.ok_or_else(|| Error::msg("CF handle not found"))?;
				db.delete_cf(&lookups_cf_handle, transaction.transaction.id().to_vec())?;

				mempool_transactions.push(transaction);
				if mempool_transactions.len() > n - 1 {
					break;
				}
			}
			Ok(mempool_transactions)
		})
		.await?
	}
}

impl MempoolBlockOperations for RocksdbMempool {
	async fn has_block(&self, block_id: block::Id) -> Result<bool, Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle =
				db.cf_handle("blocks").ok_or_else(|| Error::msg("CF handle not found"))?;
			Ok(db.get_cf(&cf_handle, block_id.to_vec())?.is_some())
		})
		.await?
	}

	async fn add_block(&self, block: Block) -> Result<(), Error> {
		let serialized_block = bcs::to_bytes(&block)?;
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle =
				db.cf_handle("blocks").ok_or_else(|| Error::msg("CF handle not found"))?;
			db.put_cf(&cf_handle, block.id().to_vec(), &serialized_block)?;
			Ok(())
		})
		.await?
	}

	async fn remove_block(&self, block_id: block::Id) -> Result<(), Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle =
				db.cf_handle("blocks").ok_or_else(|| Error::msg("CF handle not found"))?;
			db.delete_cf(&cf_handle, block_id.to_vec())?;
			Ok(())
		})
		.await?
	}

	async fn get_block(&self, block_id: block::Id) -> Result<Option<Block>, Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle =
				db.cf_handle("blocks").ok_or_else(|| Error::msg("CF handle not found"))?;
			let serialized_block = db.get_cf(&cf_handle, block_id.to_vec())?;
			match serialized_block {
				Some(serialized_block) => {
					let block: Block = bcs::from_bytes(&serialized_block)?;
					Ok(Some(block))
				}
				None => Ok(None),
			}
		})
		.await?
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_types::transaction::Transaction;
	use tempfile::tempdir;

	#[tokio::test]
	async fn test_rocksdb_mempool_basic_operations() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction = MempoolTransaction::test();
		let transaction_id = transaction.id();
		mempool.add_mempool_transaction(transaction.clone()).await?;
		assert!(mempool.has_mempool_transaction(transaction_id.clone()).await?);
		let transaction2 = mempool.get_mempool_transaction(transaction_id.clone()).await?;
		assert_eq!(Some(transaction.clone()), transaction2);
		mempool.remove_mempool_transaction(transaction_id.clone()).await?;
		assert!(!mempool.has_mempool_transaction(transaction_id.clone()).await?);

		let block = Block::test();
		let block_id = block.id();
		mempool.add_block(block.clone()).await?;
		assert!(mempool.has_block(block_id.clone()).await?);
		let block2 = mempool.get_block(block_id.clone()).await?;
		assert_eq!(Some(block.clone()), block2);
		mempool.remove_block(block_id.clone()).await?;
		assert!(!mempool.has_block(block_id.clone()).await?);

		Ok(())
	}

	#[tokio::test]
	async fn test_rocksdb_transaction_operations() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction = Transaction::test();
		let transaction_id = transaction.id();
		mempool.add_transaction(transaction.clone()).await?;
		assert!(mempool.has_transaction(transaction_id.clone()).await?);
		let transaction2 = mempool.get_transaction(transaction_id.clone()).await?;
		assert_eq!(Some(transaction.clone()), transaction2);
		mempool.remove_transaction(transaction_id.clone()).await?;
		assert!(!mempool.has_transaction(transaction_id.clone()).await?);

		Ok(())
	}

	#[tokio::test]
	async fn test_transaction_slot_based_ordering() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0), 2);
		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 0), 64);
		let transaction3 = MempoolTransaction::at_time(Transaction::new(vec![3], 0), 128);

		mempool.add_mempool_transaction(transaction2.clone()).await?;
		mempool.add_mempool_transaction(transaction1.clone()).await?;
		mempool.add_mempool_transaction(transaction3.clone()).await?;

		let transactions = mempool.pop_mempool_transactions(3).await?;
		assert_eq!(transactions[0], transaction1);
		assert_eq!(transactions[1], transaction2);
		assert_eq!(transactions[2], transaction3);

		Ok(())
	}

	#[tokio::test]
	async fn test_transaction_sequence_number_based_ordering() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0), 2);
		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 1), 2);
		let transaction3 = MempoolTransaction::at_time(Transaction::new(vec![3], 0), 64);

		mempool.add_mempool_transaction(transaction2.clone()).await?;
		mempool.add_mempool_transaction(transaction1.clone()).await?;
		mempool.add_mempool_transaction(transaction3.clone()).await?;

		let transactions = mempool.pop_mempool_transactions(3).await?;
		assert_eq!(transactions[0], transaction1);
		assert_eq!(transactions[1], transaction2);
		assert_eq!(transactions[2], transaction3);

		Ok(())
	}

	#[tokio::test]
	async fn test_slot_and_transaction_based_ordering() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0), 0);
		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 1), 0);
		let transaction3 = MempoolTransaction::at_time(Transaction::new(vec![3], 2), 0);

		mempool.add_mempool_transaction(transaction2.clone()).await?;
		mempool.add_mempool_transaction(transaction1.clone()).await?;
		mempool.add_mempool_transaction(transaction3.clone()).await?;

		let transactions = mempool.pop_mempool_transactions(3).await?;
		assert_eq!(transactions[0], transaction1);
		assert_eq!(transactions[1], transaction2);
		assert_eq!(transactions[2], transaction3);

		Ok(())
	}
}
