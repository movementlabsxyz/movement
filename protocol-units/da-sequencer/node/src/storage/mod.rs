use crate::{
	batch::{DaBatch, FullNodeTxs},
	block::{BlockHeight, SequencerBlock, MAX_SEQUENCER_BLOCK_SIZE},
	celestia::CelestiaHeight,
	error::DaSequencerError,
};
use bcs;
use movement_types::{
	block::{self, Block, BlockMetadata},
	transaction::Transaction,
};
use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};
use std::path::Path;
use std::{collections::BTreeSet, result::Result, sync::Arc};

pub mod cf {
	pub const PENDING_TRANSACTIONS: &str = "pending_transactions";
	pub const BLOCKS: &str = "blocks";
	pub const BLOCKS_BY_DIGEST: &str = "blocks_by_digest";
}

/// Used to construct the composite key: [timestamp: u64][index: u32][tx_id: [32]u8].
/// Using the composite key will naturally sort keys in lexical order in Rocksdb
#[derive(Debug, Clone, PartialEq, Eq)]
struct TxCompositeKey {
	timestamp: u64,
	index_in_batch: u32,
	tx_id: [u8; 32],
}

impl TxCompositeKey {
	/// Creates a composite key for a transaction in a batch.
	/// The key encodes the batch timestamp, transaction index, and ID,
	/// preserving correct ordering for RocksDB iteration.
	pub fn from_batch(index: usize, tx: &Transaction, timestamp: u64) -> Self {
		Self { timestamp, index_in_batch: index as u32, tx_id: tx.id().as_bytes().clone() }
	}
	/// Encode the composite key into a byte Vec.
	fn encode(&self) -> Vec<u8> {
		let mut key = Vec::with_capacity(44);
		key.extend_from_slice(&self.timestamp.to_be_bytes()); // 8 bytes
		key.extend_from_slice(&self.index_in_batch.to_be_bytes()); // 4 bytes
		key.extend_from_slice(&self.tx_id); // 32 bytes
		key
	}

	/// Decode a composite key from a byte slice.
	fn _decode(bytes: &[u8]) -> Result<Self, DaSequencerError> {
		if bytes.len() != 44 {
			return Err(DaSequencerError::StorageFormat("Invalid composite key length".into()));
		}

		let timestamp = u64::from_be_bytes(bytes[0..8].try_into()?);
		let index_in_batch = u32::from_be_bytes(bytes[8..12].try_into()?);
		let tx_id = bytes[12..44].try_into().map_err(|_| {
			DaSequencerError::StorageFormat("Failed to extract tx_id from key".into())
		})?;

		Ok(TxCompositeKey { timestamp, index_in_batch, tx_id })
	}
}

#[derive(Debug, Clone)]
pub struct Storage {
	db: Arc<DB>,
}

pub trait DaSequencerStorage {
	/// Save all batch's Tx in the pending Tx table. The batch's Tx has been verified and validated.
	fn write_batch(&self, batch: DaBatch<FullNodeTxs>) -> Result<(), DaSequencerError>;

	/// Return, if exists, the Block at the given height.
	fn get_block_at_height(
		&self,
		height: BlockHeight,
	) -> Result<Option<SequencerBlock>, DaSequencerError>;

	/// Return, if exists, the Block with specified sequencer id.
	fn get_block_with_id(&self, id: block::Id) -> Result<Option<SequencerBlock>, DaSequencerError>;

	/// Produces the next sequencer block from pending transactions.
	///
	/// - Computes the next block height.
	/// - Aggregates pending transactions in batch-timestamp order until the block is full or size limit is reached.
	/// - Removes included transactions from the pending pool.
	/// - Persists the new block and returns it.
	fn produce_next_block(&self) -> Result<Option<SequencerBlock>, DaSequencerError>;

	/// Return, if exists, the Celestia height for given block height.
	fn get_celestia_height_for_block(
		&self,
		heigh: BlockHeight,
	) -> Result<Option<CelestiaHeight>, DaSequencerError>;

	/// Set the Celestia height for a given block height.
	fn set_block_celestia_height(
		&self,
		block_heigh: BlockHeight,
		celestia_heigh: CelestiaHeight,
	) -> Result<(), DaSequencerError>;

	fn get_current_block_height(&self) -> Result<BlockHeight, DaSequencerError>;
}

impl Storage {
	pub fn try_new(path: impl AsRef<Path>) -> Result<Self, DaSequencerError> {
		let mut options = Options::default();

		options.create_if_missing(true);
		options.create_missing_column_families(true);

		let pending_transactions_cf =
			ColumnFamilyDescriptor::new(cf::PENDING_TRANSACTIONS, Options::default());
		let blocks_cf = ColumnFamilyDescriptor::new(cf::BLOCKS, Options::default());
		let blocks_by_digest_cf =
			ColumnFamilyDescriptor::new(cf::BLOCKS_BY_DIGEST, Options::default());

		let db = DB::open_cf_descriptors(
			&options,
			path,
			[pending_transactions_cf, blocks_cf, blocks_by_digest_cf],
		)
		.map_err(|e| DaSequencerError::StorageAccess(e.to_string()))?;

		Ok(Storage { db: Arc::new(db) })
	}

	fn determine_next_block_height(&self) -> Result<BlockHeight, DaSequencerError> {
		Ok(self.get_current_block_height()? + 1)
	}
}

impl DaSequencerStorage for Storage {
	fn write_batch(&self, batch: DaBatch<FullNodeTxs>) -> Result<(), DaSequencerError> {
		let cf = self.db.cf_handle(cf::PENDING_TRANSACTIONS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: pending_transactions".into())
		})?;

		let txs = batch.data();
		let mut write_batch = WriteBatch::default();

		for (i, tx) in txs.iter().enumerate() {
			let key = TxCompositeKey::from_batch(i, tx, batch.timestamp).encode();
			let value =
				bcs::to_bytes(tx).map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;
			write_batch.put_cf(&cf, key, value);
		}

		self.db
			.write(write_batch)
			.map_err(|e| DaSequencerError::RocksDbError(e.to_string()))?;

		Ok(())
	}

	fn get_block_at_height(
		&self,
		height: BlockHeight,
	) -> Result<Option<SequencerBlock>, DaSequencerError> {
		let cf = self.db.cf_handle(cf::BLOCKS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks".into())
		})?;

		let key: [u8; 8] = height.0.to_be_bytes();

		match self
			.db
			.get_cf(&cf, key)
			.map_err(|e| DaSequencerError::RocksDbError(e.to_string()))?
		{
			Some(bytes) => {
				let block = SequencerBlock::try_from(&bytes[..])?;
				Ok(Some(block))
			}
			None => Ok(None),
		}
	}

	fn get_block_with_id(&self, id: block::Id) -> Result<Option<SequencerBlock>, DaSequencerError> {
		let cf = self.db.cf_handle(cf::BLOCKS_BY_DIGEST).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks_by_digest".into())
		})?;

		let key = id;

		let height_bytes = match self
			.db
			.get_cf(&cf, key)
			.map_err(|e| DaSequencerError::RocksDbError(e.to_string()))?
		{
			Some(bytes) => bytes,
			None => return Ok(None),
		};

		let height =
			BlockHeight(height_bytes.try_into().map(u64::from_be_bytes).map_err(|_| {
				DaSequencerError::StorageFormat(
					"Invalid height byte length in digest mapping".into(),
				)
			})?);

		self.get_block_at_height(height)
	}

	fn produce_next_block(&self) -> Result<Option<SequencerBlock>, DaSequencerError> {
		let cf_pending = self.db.cf_handle(cf::PENDING_TRANSACTIONS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: pending_transactions".into())
		})?;

		let iter = self.db.iterator_cf(&cf_pending, rocksdb::IteratorMode::Start);

		let mut selected_txs = Vec::new();
		let mut keys_to_delete = Vec::new();
		let mut total_size: u64 = 0;

		for item in iter {
			let (key, value) = item.map_err(|e| DaSequencerError::RocksDbError(e.to_string()))?;
			let tx_size = value.len() as u64;

			if total_size + tx_size > MAX_SEQUENCER_BLOCK_SIZE {
				break;
			}

			let tx: Transaction = bcs::from_bytes(&value)
				.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;

			total_size += tx_size;
			selected_txs.push(tx);
			keys_to_delete.push(key.into_vec());
		}

		if selected_txs.is_empty() {
			return Ok(None);
		}

		let height = self.determine_next_block_height()?;

		let parent_id = match height.0 {
			0 | 1 => block::Id::genesis_block(),
			_ => {
				let parent_block = self.get_block_at_height(height.parent())?.ok_or_else(|| {
					DaSequencerError::StorageFormat("Missing parent block".into())
				})?;
				parent_block.id()
			}
		};

		let tx_set: BTreeSet<_> = selected_txs.into_iter().collect();
		let block = Block::new(BlockMetadata::default(), parent_id, tx_set);
		let sequencer_block = SequencerBlock::try_new(height, block)?;
		tracing::info!(
			"Producing new block: id:{} height:{} nb Tx:{}",
			sequencer_block.id(),
			sequencer_block.height().0,
			sequencer_block.len()
		);

		// Save the block and clean up pending txs
		self.save_block(&sequencer_block, Some(keys_to_delete))?;

		Ok(Some(sequencer_block))
	}

	fn get_celestia_height_for_block(
		&self,
		_height: BlockHeight,
	) -> Result<Option<CelestiaHeight>, DaSequencerError> {
		todo!();
	}

	fn set_block_celestia_height(
		&self,
		_block_height: BlockHeight,
		_celestia_height: CelestiaHeight,
	) -> Result<(), DaSequencerError> {
		todo!()
	}

	fn get_current_block_height(&self) -> Result<BlockHeight, DaSequencerError> {
		let cf = self.db.cf_handle(cf::BLOCKS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks".into())
		})?;

		let mut iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::End);
		if let Some(Ok((key, _value))) = iter.next() {
			if key.len() != 8 {
				return Err(DaSequencerError::StorageFormat(
					"Invalid block height key length".into(),
				));
			}

			let mut arr = [0u8; 8];
			arr.copy_from_slice(&key);
			let last_height = u64::from_be_bytes(arr);
			Ok(BlockHeight(last_height))
		} else {
			Ok(BlockHeight(0))
		}
	}
}

impl Storage {
	/// Saves the given block to storage by height and digest.
	/// Optionally deletes pending transaction keys from the DB.
	pub fn save_block(
		&self,
		block: &SequencerBlock,
		delete_keys: Option<Vec<Vec<u8>>>,
	) -> Result<(), DaSequencerError> {
		let block_bytes: Vec<u8> = block.try_into()?;

		let cf_blocks = self.db.cf_handle(cf::BLOCKS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks".into())
		})?;

		let cf_digests = self.db.cf_handle(cf::BLOCKS_BY_DIGEST).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks_by_digest".into())
		})?;

		let mut write_batch = WriteBatch::default();
		let height_key = block.height().0.to_be_bytes();

		write_batch.put_cf(&cf_blocks, height_key, &block_bytes);
		write_batch.put_cf(&cf_digests, block.id(), &height_key);

		if let Some(keys) = delete_keys {
			let cf_pending = self.db.cf_handle(cf::PENDING_TRANSACTIONS).ok_or_else(|| {
				DaSequencerError::StorageAccess(
					"Missing column family: pending_transactions".into(),
				)
			})?;

			for key in keys {
				write_batch.delete_cf(&cf_pending, key);
			}
		}

		self.db
			.write(write_batch)
			.map_err(|e| DaSequencerError::RocksDbError(e.to_string()))?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::batch::FullNodeTxs;

	use super::*;
	use bcs;
	use tempfile::TempDir;

	#[test]
	fn test_try_new_creates_storage_successfully() {
		let temp_dir = TempDir::new().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path);
		assert!(storage.is_ok(), "Expected Ok(Storage), got Err: {:?}", storage);
	}

	#[test]
	fn test_try_new_invalid_path_should_fail() {
		let result = Storage::try_new("");
		assert!(result.is_err());
	}

	#[test]
	fn test_write_batch_persists_transaction() {
		use crate::batch::DaBatch;
		use movement_types::transaction::Transaction;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		let tx = Transaction::test_only_new(b"test data".to_vec(), 1, 123);
		let tx_id = tx.id();

		let txs = FullNodeTxs::new(vec![tx.clone()]);
		let batch = DaBatch::test_only_new(txs);
		let batch_ts = batch.timestamp;

		storage.write_batch(batch).expect("write_batch failed");

		let cf = storage
			.db
			.cf_handle(cf::PENDING_TRANSACTIONS)
			.expect("missing pending_transactions CF");

		// Scan all keys to find the one that contains this tx_id
		let mut found = false;
		let iter = storage.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
		for item in iter {
			let (key, value) = item.expect("iterator error");

			// Composite key layout: [timestamp: u64][index: u32][tx_id: [u8; 32]]
			if key.len() != 8 + 4 + 32 {
				continue;
			}

			let stored_tx_id = &key[12..]; // last 32 bytes
			if stored_tx_id == tx_id.as_ref() {
				// Check timestamp
				let ts = u64::from_be_bytes(key[0..8].try_into().unwrap());
				assert_eq!(ts, batch_ts);

				let stored_bytes = value;
				let stored_tx: Transaction = bcs::from_bytes(&stored_bytes)
					.expect("failed to deserialize stored transaction");

				assert_eq!(stored_tx.id(), tx_id);
				assert_eq!(stored_tx.sequence_number(), 123);
				assert_eq!(stored_tx.application_priority(), 1);
				assert_eq!(stored_tx.data(), b"test data");
				found = true;
				break;
			}
		}

		assert!(found, "Did not find transaction in pending CF");
	}

	#[test]
	fn test_write_batch_naturally_orders_lexicographically_with_composite_key() {
		use crate::batch::DaBatch;
		use movement_types::transaction::Transaction;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		// Create transactions
		let tx1 = Transaction::test_only_new(b"tx1".to_vec(), 1, 1);
		let tx2 = Transaction::test_only_new(b"tx2".to_vec(), 1, 2);
		let tx3 = Transaction::test_only_new(b"tx3".to_vec(), 1, 3);

		// Create the OLDER batch first
		let older_batch = DaBatch::test_only_new(FullNodeTxs::new(vec![tx1.clone(), tx2.clone()]));
		std::thread::sleep(std::time::Duration::from_millis(1)); // ensure newer timestamp
		let newer_batch = DaBatch::test_only_new(FullNodeTxs::new(vec![tx3.clone()]));

		// Write batches to DB
		storage.write_batch(older_batch).expect("write_batch (older) failed");
		storage.write_batch(newer_batch).expect("write_batch (newer) failed");

		let cf = storage
			.db
			.cf_handle(cf::PENDING_TRANSACTIONS)
			.expect("missing pending_transactions CF");

		let iter = storage.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

		let mut ordered_ids = Vec::new();

		for item in iter {
			let (key, value) = item.expect("iterator error");

			assert_eq!(key.len(), 44, "Composite key length should be 44 bytes");

			let tx: Transaction =
				bcs::from_bytes(&value).expect("failed to deserialize transaction");
			ordered_ids.push(tx.id());
		}

		assert_eq!(ordered_ids.len(), 3, "Expected 3 transactions in DB");

		assert_eq!(ordered_ids[0], tx1.id(), "tx1 (older batch, index 0) should come first");
		assert_eq!(ordered_ids[1], tx2.id(), "tx2 (older batch, index 1) should be second");
		assert_eq!(ordered_ids[2], tx3.id(), "tx3 (newer batch) should be last");
	}

	#[test]
	fn save_block_should_persist_block_and_remove_pending_transactions() {
		use crate::batch::DaBatch;
		use crate::block::SequencerBlock;
		use movement_types::{block::Block, transaction::Transaction};
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		// Create a transaction and write it as pending
		let tx = Transaction::test_only_new(b"save test".to_vec(), 0, 1);
		let tx_id = tx.id();
		let batch = DaBatch::test_only_new(FullNodeTxs::new(vec![tx.clone()]));
		let composite_key = {
			let mut key = Vec::with_capacity(44);
			key.extend_from_slice(&batch.timestamp.to_be_bytes());
			key.extend_from_slice(&0u32.to_be_bytes()); // index = 0
			key.extend_from_slice(tx_id.as_bytes());
			key
		};

		storage.write_batch(batch).expect("failed to write batch");

		// Construct a dummy block to save
		let height = BlockHeight(1);
		let block = Block::new(BlockMetadata::default(), block::Id::default(), [tx.clone()].into());
		let sequencer_block = SequencerBlock::try_new(height, block).expect("valid block");

		// Save the block and remove the pending tx
		storage
			.save_block(&sequencer_block, Some(vec![composite_key.clone()]))
			.expect("save_block failed");

		// Check the block exists at the correct height
		let fetched = storage.get_block_at_height(height).expect("get_block_at_height failed");
		assert!(fetched.is_some(), "Expected saved block to exist");
		assert_eq!(fetched.unwrap(), sequencer_block);

		// Check it can be fetched by digest
		let id = sequencer_block.id();
		let fetched_by_digest = storage.get_block_with_id(id).expect("get by digest failed");
		assert!(fetched_by_digest.is_some(), "Expected block by digest");
		assert_eq!(fetched_by_digest.unwrap(), sequencer_block);

		// Check that the pending transaction has been removed
		let cf_pending = storage
			.db
			.cf_handle(cf::PENDING_TRANSACTIONS)
			.expect("missing 'pending_transactions' CF");
		let maybe_tx = storage.db.get_cf(&cf_pending, composite_key).expect("read failed");
		assert!(maybe_tx.is_none(), "Expected pending transaction to be removed");
	}

	#[test]
	fn test_get_block_at_height_returns_correct_block() {
		use crate::block::{BlockHeight, SequencerBlock};
		use movement_types::block::Block;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		let block_height = BlockHeight(42);
		let dummy_block = Block::default();
		let sequencer_block = SequencerBlock::try_new(block_height, dummy_block.clone()).unwrap();

		let encoded_block =
			bcs::to_bytes(&sequencer_block).expect("failed to serialize SequencerBlock");

		let cf = storage.db.cf_handle(cf::BLOCKS).expect("missing 'blocks' column family");
		let key = block_height.0.to_be_bytes();
		storage.db.put_cf(&cf, key, encoded_block).expect("failed to write to db");

		let result = storage.get_block_at_height(block_height).expect("get_block_at_height failed");

		assert!(result.is_some(), "expected Some(block), got None");
		let fetched_block = result.unwrap();
		assert_eq!(fetched_block, sequencer_block);
	}

	#[test]
	fn test_get_block_with_digest_returns_correct_block() {
		use crate::block::{BlockHeight, SequencerBlock};
		use movement_types::block::Block;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		let block_height = BlockHeight(99);
		let dummy_block = Block::default();
		let sequencer_block = SequencerBlock::try_new(block_height, dummy_block.clone()).unwrap();
		let id = sequencer_block.id();

		let encoded_block =
			bcs::to_bytes(&sequencer_block).expect("failed to serialize SequencerBlock");

		let cf_blocks = storage.db.cf_handle(cf::BLOCKS).expect("missing 'blocks' column family");
		let key = block_height.0.to_be_bytes();
		storage
			.db
			.put_cf(&cf_blocks, key, encoded_block)
			.expect("failed to write to BLOCKS CF");

		let cf_digests =
			storage.db.cf_handle(cf::BLOCKS_BY_DIGEST).expect("missing 'block_digests' CF");
		storage.db.put_cf(&cf_digests, id, key).expect("failed to write digest mapping");

		let result = storage.get_block_with_id(id).expect("get_block_with_digest failed");

		assert!(result.is_some(), "Expected Some(block), got None");
		let fetched_block = result.unwrap();
		assert_eq!(fetched_block, sequencer_block);

		let stored_height_bytes = storage
			.db
			.get_cf(&cf_digests, id)
			.expect("failed to read digest mapping")
			.expect("digest not found");
		assert_eq!(stored_height_bytes, key);
	}

	#[test]
	fn test_get_block_with_digest_returns_none_for_unknown_digest() {
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		let fake_id = block::Id::new([0u8; 32]);
		let result = storage.get_block_with_id(fake_id);

		assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result);
		assert!(result.unwrap().is_none(), "Expected None for unknown digest, got Some");
	}

	#[test]
	fn test_produce_next_block_generates_block_and_clears_pending_tx() {
		use crate::batch::DaBatch;
		use movement_types::transaction::Transaction;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		let tx = Transaction::test_only_new(b"test data".to_vec(), 0, 1);
		let tx_assert = tx.clone();
		let tx_id = tx.id();

		let txs = FullNodeTxs::new(vec![tx]);
		let batch = DaBatch::test_only_new(txs);
		storage.write_batch(batch).expect("failed to write batch");
		let maybe_block = storage.produce_next_block().expect("produce_next_block failed");

		assert!(maybe_block.is_some(), "Expected Some(block), got None");
		let block = maybe_block.unwrap();
		let transactions: Vec<_> = block.transactions().cloned().collect();
		assert_eq!(transactions.len(), 1, "Expected 1 transaction in block");
		assert_eq!(transactions[0], tx_assert, "Transaction in block does not match original");

		let cf_pending = storage
			.db
			.cf_handle(cf::PENDING_TRANSACTIONS)
			.expect("missing 'pending_transactions' CF");
		let maybe_tx = storage.db.get_cf(&cf_pending, tx_id).expect("failed to read pending tx");
		assert!(maybe_tx.is_none(), "Pending transaction was not cleared");

		let cf_blocks = storage.db.cf_handle(cf::BLOCKS).expect("missing 'blocks' CF");
		let height_key = block.height().0.to_be_bytes();
		let stored_bytes = storage
			.db
			.get_cf(&cf_blocks, height_key)
			.expect("failed to read stored block")
			.expect("block not found");

		let stored_block: SequencerBlock =
			bcs::from_bytes(&stored_bytes).expect("failed to deserialize stored block");
		assert_eq!(stored_block, block, "Stored block does not match produced block");
	}

	#[test]
	fn test_produce_block_fits_all_tx_and_clears_pending() {
		use crate::batch::DaBatch;
		use movement_types::transaction::Transaction;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		// Write 10 transactions
		let txs: Vec<_> = (0..10)
			.map(|i| Transaction::test_only_new(format!("data-{i}").into_bytes(), 1, i))
			.collect();
		let batch = DaBatch::test_only_new(FullNodeTxs::new(txs.clone()));
		storage.write_batch(batch).expect("failed to write batch");

		// Produce the block
		let block = storage
			.produce_next_block()
			.expect("produce_next_block failed")
			.expect("expected Some(block)");

		assert_eq!(block.height().0, 1, "Expected height to be 1");

		// Check no pending tx remain
		let cf = storage.db.cf_handle(cf::PENDING_TRANSACTIONS).expect("missing pending CF");
		let mut iter = storage.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
		assert!(iter.next().is_none(), "Expected no pending txs");

		// Check block contains all txs
		let block_tx_ids: BTreeSet<_> = block.transactions().map(|tx| tx.id()).collect();
		let original_tx_ids: BTreeSet<_> = txs.into_iter().map(|tx| tx.id()).collect();
		assert_eq!(block_tx_ids, original_tx_ids);

		// Height 1 exists
		let retrieved = storage.get_block_at_height(BlockHeight(1)).unwrap();
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap(), block);

		// Heights 0 and 2 do not exist
		assert!(storage.get_block_at_height(BlockHeight(0)).unwrap().is_none());
		assert!(storage.get_block_at_height(BlockHeight(2)).unwrap().is_none());
	}
}
