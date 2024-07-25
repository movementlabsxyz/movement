use anyhow::Error;
use bcs;
use mempool_util::{MempoolBlockOperations, MempoolTransaction, MempoolTransactionOperations};
use movement_types::{Block, Id};
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct RocksdbMempool {
        db: Arc<RwLock<DB>>,
}

impl RocksdbMempool {
        pub fn try_new(path: &str) -> Result<Self, Error> {
                let mut options = Options::default();
                options.create_if_missing(true);
                options.create_missing_column_families(true);

                let mempool_transactions_cf = ColumnFamilyDescriptor::new("mempool_transactions", Options::default());
                let transaction_truths_cf = ColumnFamilyDescriptor::new("transaction_truths", Options::default());
                let blocks_cf = ColumnFamilyDescriptor::new("blocks", Options::default());
                let transaction_lookups_cf = ColumnFamilyDescriptor::new("transaction_lookups", Options::default());

                let db = DB::open_cf_descriptors(
                        &options,
                        path,
                        vec![mempool_transactions_cf, transaction_truths_cf, blocks_cf, transaction_lookups_cf],
                )
                .map_err(|e| Error::new(e))?;

                Ok(RocksdbMempool { db: Arc::new(RwLock::new(db)) })
        }

        pub fn construct_mempool_transaction_key(transaction: &MempoolTransaction) -> String {
                // pad to 32 characters for slot_seconds
                let slot_seconds_str = format!("{:032}", transaction.timestamp);

                // pad to 32 characters for sequence number
                let sequence_number_str = format!("{:032}", transaction.transaction.sequence_number);

		// Assuming transaction.transaction.id() returns a hex string of length 32
                let transaction_id_hex = transaction.transaction.id(); // This should be a String of hex characters

		// Concatenate the two parts to form a 80-character hex string key
                let key = format!("{}:{}:{}", slot_seconds_str, sequence_number_str, transaction_id_hex);

                key
        }

	/// Helper function to retrieve the key for mempool transaction from the lookup table.
        async fn get_mempool_transaction_key(
                &self,
                transaction_id: &Id,
        ) -> Result<Option<Vec<u8>>, Error> {
                let db = self.db.read().await;
                let cf_handle = db
                        .cf_handle("transaction_lookups")
                        .ok_or_else(|| Error::msg("CF handle not found"))?;
                db.get_cf(&cf_handle, transaction_id.to_vec()).map_err(|e| Error::new(e))
        }
}

impl MempoolTransactionOperations for RocksdbMempool {
        async fn has_mempool_transaction(&self, transaction_id: Id) -> Result<bool, Error> {
                let key = self.get_mempool_transaction_key(&transaction_id).await?;
                match key {
                        Some(k) => {
                                let db = self.db.read().await;
                                let cf_handle = db
                                        .cf_handle("mempool_transactions")
                                        .ok_or_else(|| Error::msg("CF handle not found"))?;
                                Ok(db.get_cf(&cf_handle, k)?.is_some())
                        }
                        None => Ok(false),
                }
        }

        async fn add_mempool_transaction(&self, tx: MempoolTransaction) -> Result<(), Error> {
                let serialized_tx = bcs::to_bytes(&tx)?;
                println!("Serialized transaction: {:?}", serialized_tx);

                let db = self.db.write().await;
                let mempool_transactions_cf_handle = db
                        .cf_handle("mempool_transactions")
                        .ok_or_else(|| Error::msg("CF handle not found"))?;
                let transaction_lookups_cf_handle = db
                        .cf_handle("transaction_lookups")
                        .ok_or_else(|| Error::msg("CF handle not found"))?;

                let key = Self::construct_mempool_transaction_key(&tx);
                println!("Constructed key: {}", key);

                db.put_cf(&mempool_transactions_cf_handle, &key, &serialized_tx)?;
                db.put_cf(&transaction_lookups_cf_handle, tx.transaction.id().to_vec(), &key)?;

                Ok(())
        }

        async fn remove_mempool_transaction(&self, transaction_id: Id) -> Result<(), Error> {
                let key = self.get_mempool_transaction_key(&transaction_id).await?;

                match key {
                        Some(k) => {
                                let db = self.db.write().await;
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
                Ok(())
        }

	// Updated method signatures and implementations go here
        async fn get_mempool_transaction(
                &self,
                transaction_id: Id,
        ) -> Result<Option<MempoolTransaction>, Error> {
                let key = match self.get_mempool_transaction_key(&transaction_id).await? {
                        Some(k) => k,
                        None => return Ok(None), // If no key found in lookup, return None
                };
                let db = self.db.read().await;
                let cf_handle = db
                        .cf_handle("mempool_transactions")
                        .ok_or_else(|| Error::msg("CF handle not found"))?;
                match db.get_cf(&cf_handle, &key)? {
                        Some(serialized_tx) => {
                                println!("Serialized transaction from DB: {:?}", serialized_tx);
                                let tx: MempoolTransaction = bcs::from_bytes(&serialized_tx)?;
                                println!("Deserialized transaction: {:?}", tx);
                                Ok(Some(tx))
                        }
                        None => Ok(None),
                }
        }

        async fn pop_mempool_transaction(&self) -> Result<Option<MempoolTransaction>, Error> {
                let db = self.db.write().await;
                let cf_handle = db
                        .cf_handle("mempool_transactions")
                        .ok_or_else(|| Error::msg("CF handle not found"))?;
                let mut iter = db.iterator_cf(&cf_handle, rocksdb::IteratorMode::Start);

                match iter.next() {
                        None => return Ok(None), // No transactions to pop
                        Some(res) => {
                                let (key, value) = res?;
                                let tx: MempoolTransaction = bcs::from_bytes(&value)?;
                                println!("Deserialized transaction: {:?}", tx);
                                db.delete_cf(&cf_handle, &key)?;

				// Optionally, remove from the lookup table as well
                                let lookups_cf_handle = db
                                        .cf_handle("transaction_lookups")
                                        .ok_or_else(|| Error::msg("CF handle not found"))?;
                                db.delete_cf(&lookups_cf_handle, tx.transaction.id().to_vec())?;

                                Ok(Some(tx))
                        }
                }
        }
}

impl MempoolBlockOperations for RocksdbMempool {
        async fn has_block(&self, block_id: Id) -> Result<bool, Error> {
                let db = self.db.read().await;
                let cf_handle = db.cf_handle("blocks").ok_or_else(|| Error::msg("CF handle not found"))?;
                Ok(db.get_cf(&cf_handle, block_id.to_vec())?.is_some())
        }

        async fn add_block(&self, block: Block) -> Result<(), Error> {
                let serialized_block = bcs::to_bytes(&block)?;
                let db = self.db.write().await;
                let cf_handle = db.cf_handle("blocks").ok_or_else(|| Error::msg("CF handle not found"))?;
                db.put_cf(&cf_handle, block.id().to_vec(), &serialized_block)?;
                Ok(())
        }

        async fn remove_block(&self, block_id: Id) -> Result<(), Error> {
                let db = self.db.write().await;
                let cf_handle = db.cf_handle("blocks").ok_or_else(|| Error::msg("CF handle not found"))?;
                db.delete_cf(&cf_handle, block_id.to_vec())?;
                Ok(())
        }

        async fn get_block(&self, block_id: Id) -> Result<Option<Block>, Error> {
                let db = self.db.read().await;
                let cf_handle = db.cf_handle("blocks").ok_or_else(|| Error::msg("CF handle not found"))?;
                let serialized_block = db.get_cf(&cf_handle, block_id.to_vec())?;
                match serialized_block {
                        Some(serialized_block) => {
                                let block: Block = bcs::from_bytes(&serialized_block)?;
                                Ok(Some(block))
                        }
                        None => Ok(None),
                }
        }
}

#[cfg(test)]
pub mod test {
        use super::*;
        use movement_types::Transaction;
        use tempfile::tempdir;

        #[tokio::test]
        async fn test_rocksdb_mempool_basic_operations() -> Result<(), Error> {
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
        async fn test_rocksdb_transaction_operations() -> Result<(), Error> {
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
        async fn test_transaction_slot_based_ordering() -> Result<(), Error> {
                let temp_dir = tempdir().unwrap();
                let path = temp_dir.path().to_str().unwrap();
                let mempool = RocksdbMempool::try_new(path)?;

                let tx1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0), 2);
                let tx2 = MempoolTransaction::at_time(Transaction::new(vec![2], 0), 64);
                let tx3 = MempoolTransaction::at_time(Transaction::new(vec![3], 0), 128);

                mempool.add_mempool_transaction(tx2.clone()).await?;
                mempool.add_mempool_transaction(tx1.clone()).await?;
                mempool.add_mempool_transaction(tx3.clone()).await?;

                let txs = mempool.pop_mempool_transactions(3).await?;
                assert_eq!(txs[0], tx1);
                assert_eq!(txs[1], tx2);
                assert_eq!(txs[2], tx3);

                Ok(())
        }

        #[tokio::test]
        async fn test_transaction_sequence_number_based_ordering() -> Result<(), Error> {
                let temp_dir = tempdir().unwrap();
                let path = temp_dir.path().to_str().unwrap();
                let mempool = RocksdbMempool::try_new(path)?;

                let tx1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0), 2);
                let tx2 = MempoolTransaction::at_time(Transaction::new(vec![2], 1), 2);
                let tx3 = MempoolTransaction::at_time(Transaction::new(vec![3], 0), 64);

                mempool.add_mempool_transaction(tx2.clone()).await?;
                mempool.add_mempool_transaction(tx1.clone()).await?;
                mempool.add_mempool_transaction(tx3.clone()).await?;

                let txs = mempool.pop_mempool_transactions(3).await?;
                assert_eq!(txs[0], tx1);
                assert_eq!(txs[1], tx2);
                assert_eq!(txs[2], tx3);

                Ok(())
        }

        #[tokio::test]
        async fn test_slot_and_transaction_based_ordering() -> Result<(), Error> {
                let temp_dir = tempdir().unwrap();
                let path = temp_dir.path().to_str().unwrap();
                let mempool = RocksdbMempool::try_new(path)?;

                let tx1 = MempoolTransaction::at_time(Transaction::new(vec![1], 0), 0);
                let tx2 = MempoolTransaction::at_time(Transaction::new(vec![2], 1), 0);
                let tx3 = MempoolTransaction::at_time(Transaction::new(vec![3], 2), 0);

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
