use anyhow::Error;
use bcs;
use mempool_util::{MempoolBlockOperations, MempoolTransaction, MempoolTransactionOperations};
use movement_types::{
	block::{self, Block},
	transaction,
};
use rocksdb::{ColumnFamilyDescriptor, IteratorMode, Options, ReadOptions, WriteBatch, DB};
use std::fmt::Write;
use std::sync::Arc;

mod cf {
	pub const MEMPOOL_TRANSACTIONS: &str = "mempool_transactions";
	pub const BLOCKS: &str = "blocks";
	pub const TRANSACTION_LOOKUPS: &str = "transaction_lookups";
	pub const TRANSACTION_TIMELINE: &str = "transaction_timeline";
}

#[derive(Debug, Clone)]
pub struct RocksdbMempool {
	db: Arc<DB>,
}

fn construct_mempool_transaction_key(transaction: &MempoolTransaction) -> String {
	// Pre-allocate a string with the required capacity
	let mut key = String::with_capacity(32 + 1 + 32 + 1 + 32 + 1 + 64);
	// Write key components. The numbers are zero-padded to 32 characters.
	key.write_fmt(format_args!(
		"{:032}:{:032}:{:032}:{}",
		transaction.transaction.application_priority(),
		transaction.timestamp,
		transaction.transaction.sequence_number(),
		transaction.transaction.id(),
	))
	.unwrap(); // write to String never fails
	key
}

fn construct_transaction_timeline_key(transaction: &MempoolTransaction) -> String {
	// Pre-allocate a string with the required capacity
	let mut key = String::with_capacity(32 + 1 + 64);
	// Write key components. The numbers are zero-padded to 32 characters.
	key.write_fmt(format_args!("{:032}:{}", transaction.timestamp, transaction.transaction.id()))
		.unwrap(); // write to String never fails
	key
}

fn construct_timeline_threshold_key(timestamp_threshold: u64) -> String {
	let mut key = String::with_capacity(32 + 1);
	key.write_fmt(format_args!("{:032}:", timestamp_threshold)).unwrap();
	key
}

impl RocksdbMempool {
	pub fn try_new(path: &str) -> Result<Self, Error> {
		let mut options = Options::default();
		options.create_if_missing(true);
		options.create_missing_column_families(true);

		let mempool_transactions_cf =
			ColumnFamilyDescriptor::new(cf::MEMPOOL_TRANSACTIONS, Options::default());
		let blocks_cf = ColumnFamilyDescriptor::new(cf::BLOCKS, Options::default());
		let transaction_lookups_cf =
			ColumnFamilyDescriptor::new(cf::TRANSACTION_LOOKUPS, Options::default());
		let transaction_timeline_cf =
			ColumnFamilyDescriptor::new(cf::TRANSACTION_TIMELINE, Options::default());

		let db = DB::open_cf_descriptors(
			&options,
			path,
			[mempool_transactions_cf, blocks_cf, transaction_lookups_cf, transaction_timeline_cf],
		)
		.map_err(|e| Error::new(e))?;

		Ok(RocksdbMempool { db: Arc::new(db) })
	}

	fn internal_get_mempool_transaction_key(
		db: &DB,
		transaction_id: transaction::Id,
	) -> Result<Option<Vec<u8>>, Error> {
		let cf_handle = db
			.cf_handle(cf::TRANSACTION_LOOKUPS)
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
			Self::internal_get_mempool_transaction_key(&db, transaction_id)
		})
		.await?
	}

	fn internal_has_mempool_transaction(
		db: &DB,
		transaction_id: transaction::Id,
	) -> Result<bool, Error> {
		let key = Self::internal_get_mempool_transaction_key(&db, transaction_id)?;
		match key {
			Some(k) => {
				let cf_handle = db
					.cf_handle(cf::MEMPOOL_TRANSACTIONS)
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
			Self::internal_has_mempool_transaction(&db, transaction_id)
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
				.cf_handle(cf::MEMPOOL_TRANSACTIONS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let transaction_lookups_cf_handle = db
				.cf_handle(cf::TRANSACTION_LOOKUPS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let transaction_timeline_cf_handle = db
				.cf_handle(cf::TRANSACTION_TIMELINE)
				.ok_or_else(|| Error::msg("CF handle not found"))?;

			// Add the transactions and update the lookup table and the GC timeline
			// atomically in a single write batch.
			// https://github.com/movementlabsxyz/movement/issues/322

			let mut batch = WriteBatch::default();

			for transaction in transactions {
				if Self::internal_has_mempool_transaction(&db, transaction.transaction.id())? {
					continue;
				}

				let serialized_transaction = bcs::to_bytes(&transaction)?;
				let key = construct_mempool_transaction_key(&transaction);
				batch.put_cf(&mempool_transactions_cf_handle, &key, &serialized_transaction);
				batch.put_cf(
					&transaction_lookups_cf_handle,
					transaction.transaction.id().to_vec(),
					&key,
				);
				batch.put_cf(
					&transaction_timeline_cf_handle,
					&construct_transaction_timeline_key(&transaction),
					&key,
				);
			}

			db.write(batch)?;

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
				.cf_handle(cf::MEMPOOL_TRANSACTIONS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let transaction_lookups_cf_handle = db
				.cf_handle(cf::TRANSACTION_LOOKUPS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let transaction_timeline_cf_handle = db
				.cf_handle(cf::TRANSACTION_TIMELINE)
				.ok_or_else(|| Error::msg("CF handle not found"))?;

			// Add the transaction and update the lookup table and the GC timeline
			// atomically in a single write batch.
			// https://github.com/movementlabsxyz/movement/issues/322

			let mut batch = WriteBatch::default();

			let key = construct_mempool_transaction_key(&transaction);
			batch.put_cf(&mempool_transactions_cf_handle, &key, &serialized_transaction);
			batch.put_cf(
				&transaction_lookups_cf_handle,
				transaction.transaction.id().to_vec(),
				&key,
			);
			batch.put_cf(
				&transaction_timeline_cf_handle,
				&construct_transaction_timeline_key(&transaction),
				&key,
			);

			db.write(batch)?;

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
						.cf_handle(cf::MEMPOOL_TRANSACTIONS)
						.ok_or_else(|| Error::msg("CF handle not found"))?;
					let lookups_cf_handle = db
						.cf_handle(cf::TRANSACTION_LOOKUPS)
						.ok_or_else(|| Error::msg("CF handle not found"))?;

					// Remove the transaction and its entry in the lookup table
					// atomically in a single write batch.
					// https://github.com/movementlabsxyz/movement/issues/322
					// Note that the timeline entry is not removed. It should be
					// eventually collected in a GC pass.

					let mut batch = WriteBatch::default();
					batch.delete_cf(&cf_handle, k);
					batch.delete_cf(&lookups_cf_handle, transaction_id.to_vec());
					db.write(batch)?;
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
				.cf_handle(cf::MEMPOOL_TRANSACTIONS)
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
				.cf_handle(cf::MEMPOOL_TRANSACTIONS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let lookups_cf_handle = db
				.cf_handle(cf::TRANSACTION_LOOKUPS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let mut iter = db.iterator_cf(&cf_handle, IteratorMode::Start);

			match iter.next() {
				None => return Ok(None), // No transactions to pop
				Some(res) => {
					let (key, value) = res?;
					let transaction: MempoolTransaction = bcs::from_bytes(&value)?;

					// Remove the transaction and its entry in the lookup table
					// atomically in a single write batch.
					// https://github.com/movementlabsxyz/movement/issues/322
					// Note that the timeline entry is not removed. It should be
					// eventually collected in a GC pass.

					let mut batch = WriteBatch::default();
					batch.delete_cf(&cf_handle, &key);
					batch.delete_cf(&lookups_cf_handle, transaction.transaction.id().to_vec());
					db.write(batch)?;

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
				.cf_handle(cf::MEMPOOL_TRANSACTIONS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let lookups_cf_handle = db
				.cf_handle(cf::TRANSACTION_LOOKUPS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;

			// Remove the transactions and their lookup table entries
			// atomically in a single write batch.
			// https://github.com/movementlabsxyz/movement/issues/322

			let mut iter = db.iterator_cf(&cf_handle, IteratorMode::Start);
			let mut batch = WriteBatch::default();
			let mut mempool_transactions = Vec::with_capacity(n as usize);
			while let Some(res) = iter.next() {
				let (key, value) = res?;
				let transaction: MempoolTransaction = bcs::from_bytes(&value)?;

				batch.delete_cf(&cf_handle, &key);
				batch.delete_cf(&lookups_cf_handle, transaction.transaction.id().to_vec());

				mempool_transactions.push(transaction);
				if mempool_transactions.len() >= n {
					break;
				}
			}
			db.write(batch)?;

			Ok(mempool_transactions)
		})
		.await?
	}

	async fn gc_mempool_transactions(
		&self,
		timestamp_threshold: u64,
	) -> Result<u64, anyhow::Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle = db
				.cf_handle(cf::MEMPOOL_TRANSACTIONS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let timeline_cf_handle = db
				.cf_handle(cf::TRANSACTION_TIMELINE)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let lookups_cf_handle = db
				.cf_handle(cf::TRANSACTION_LOOKUPS)
				.ok_or_else(|| Error::msg("CF handle not found"))?;
			let mut read_options = ReadOptions::default();
			read_options
				.set_iterate_upper_bound(construct_timeline_threshold_key(timestamp_threshold));
			let mut iter =
				db.iterator_cf_opt(&timeline_cf_handle, read_options, IteratorMode::Start);
			let mut transaction_count = 0;
			let mut batch = WriteBatch::default();

			while let Some(res) = iter.next() {
				let (timeline_key, key) = res?;

				batch.delete_cf(&timeline_cf_handle, &timeline_key);

				let value = match db.get_cf(&cf_handle, &key)? {
					Some(value) => value,
					None => {
						// The transaction has been removed
						continue;
					}
				};
				let transaction: MempoolTransaction = bcs::from_bytes(&value)?;

				batch.delete_cf(&cf_handle, &key);
				batch.delete_cf(&lookups_cf_handle, transaction.transaction.id().to_vec());

				transaction_count += 1;
			}

			db.write(batch)?;

			Ok(transaction_count)
		})
		.await?
	}
}

impl MempoolBlockOperations for RocksdbMempool {
	async fn has_block(&self, block_id: block::Id) -> Result<bool, Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle =
				db.cf_handle(cf::BLOCKS).ok_or_else(|| Error::msg("CF handle not found"))?;
			Ok(db.get_cf(&cf_handle, block_id.to_vec())?.is_some())
		})
		.await?
	}

	async fn add_block(&self, block: Block) -> Result<(), Error> {
		let serialized_block = bcs::to_bytes(&block)?;
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle =
				db.cf_handle(cf::BLOCKS).ok_or_else(|| Error::msg("CF handle not found"))?;
			db.put_cf(&cf_handle, block.id().to_vec(), &serialized_block)?;
			Ok(())
		})
		.await?
	}

	async fn remove_block(&self, block_id: block::Id) -> Result<(), Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle =
				db.cf_handle(cf::BLOCKS).ok_or_else(|| Error::msg("CF handle not found"))?;
			db.delete_cf(&cf_handle, block_id.to_vec())?;
			Ok(())
		})
		.await?
	}

	async fn get_block(&self, block_id: block::Id) -> Result<Option<Block>, Error> {
		let db = self.db.clone();
		tokio::task::spawn_blocking(move || {
			let cf_handle =
				db.cf_handle(cf::BLOCKS).ok_or_else(|| Error::msg("CF handle not found"))?;
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
pub mod tests {

	use super::*;
	use movement_types::transaction::Transaction;
	use tempfile::tempdir;
	use tokio::time::{sleep, Duration};

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
	async fn test_rocksdb_gc() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0, 0), 2);
		let transaction1_id = transaction1.id();
		mempool.add_mempool_transaction(transaction1).await?;
		assert!(mempool.has_transaction(transaction1_id).await?);

		sleep(Duration::from_secs(2)).await;

		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 0, 0), 64);
		let transaction2_id = transaction2.id();
		let transaction2_timestamp = transaction2.timestamp;
		mempool.add_mempool_transaction(transaction2).await?;

		mempool.gc_mempool_transactions(transaction2_timestamp).await?;

		assert!(!mempool.has_transaction(transaction1_id).await?);
		assert!(mempool.has_transaction(transaction2_id).await?);

		Ok(())
	}

	#[tokio::test]
	async fn test_transaction_slot_based_ordering() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0, 0), 2);
		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 0, 0), 64);
		let transaction3 = MempoolTransaction::at_time(Transaction::new(vec![3], 0, 0), 128);

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

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0, 0), 2);
		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 0, 1), 2);
		let transaction3 = MempoolTransaction::at_time(Transaction::new(vec![3], 0, 0), 64);

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

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0, 0), 0);
		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 0, 1), 0);
		let transaction3 = MempoolTransaction::at_time(Transaction::new(vec![3], 0, 2), 0);

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
	async fn test_application_priority_based_ordering() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0, 0), 0);
		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 1, 0), 0);
		let transaction3 = MempoolTransaction::at_time(Transaction::new(vec![3], 2, 0), 0);

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
	async fn test_total_ordering() -> Result<(), Error> {
		let temp_dir = tempdir().unwrap();
		let path = temp_dir.path().to_str().unwrap();
		let mempool = RocksdbMempool::try_new(path)?;

		let transaction1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0, 0), 0);
		let transaction2 = MempoolTransaction::at_time(Transaction::new(vec![2], 0, 1), 0);
		let transaction3 = MempoolTransaction::at_time(Transaction::new(vec![3], 0, 1), 2);
		let transaction4 = MempoolTransaction::at_time(Transaction::new(vec![4], 1, 1), 2);
		let transaction5 = MempoolTransaction::at_time(Transaction::new(vec![5], 1, 2), 4);
		let transaction6 = MempoolTransaction::at_time(Transaction::new(vec![6], 1, 2), 6);

		mempool.add_mempool_transaction(transaction2.clone()).await?;
		mempool.add_mempool_transaction(transaction1.clone()).await?;
		mempool.add_mempool_transaction(transaction3.clone()).await?;
		mempool.add_mempool_transaction(transaction5.clone()).await?;
		mempool.add_mempool_transaction(transaction4.clone()).await?;
		mempool.add_mempool_transaction(transaction6.clone()).await?;

		let transactions = mempool.pop_mempool_transactions(6).await?;
		assert_eq!(transactions[0], transaction1);
		assert_eq!(transactions[1], transaction2);
		assert_eq!(transactions[2], transaction3);
		assert_eq!(transactions[3], transaction4);
		assert_eq!(transactions[4], transaction5);
		assert_eq!(transactions[5], transaction6);

		Ok(())
	}
}
