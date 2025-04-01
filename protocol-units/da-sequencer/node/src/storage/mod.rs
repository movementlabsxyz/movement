use crate::{
	batch::{DaBatch, FullNodeTxs},
	block::{BlockHeight, SequencerBlock, SequencerBlockDigest, MAX_SEQUENCER_BLOCK_SIZE},
	celestia::CelestiaHeight,
	error::DaSequencerError,
};
use bcs;
use movement_types::{
	block::{Block, BlockMetadata, Id},
	transaction::Transaction,
};
use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};
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
	fn decode(bytes: &[u8]) -> Result<Self, DaSequencerError> {
		if bytes.len() != 44 {
			return Err(DaSequencerError::StorageFormat("Invalid composite key length".into()));
		}

		let timestamp = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
		let index_in_batch = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
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

pub trait DaSequencerStorage: Clone {
	/// Save all batch's Tx in the pending Tx table. The batch's Tx has been verified and validated.
	fn write_batch(&self, batch: DaBatch<FullNodeTxs>)
		-> std::result::Result<(), DaSequencerError>;

	/// Return, if exists, the Block at the given height.
	fn get_block_at_height(
		&self,
		height: BlockHeight,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError>;

	/// Return, if exists, the Block with specified sequencer id.
	fn get_block_with_digest(
		&self,
		id: SequencerBlockDigest,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError>;

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
	) -> std::result::Result<Option<CelestiaHeight>, DaSequencerError>;

	/// Set the Celestia height for a given block height.
	fn set_block_celestia_height(
		&self,
		block_heigh: BlockHeight,
		celestia_heigh: CelestiaHeight,
	) -> std::result::Result<(), DaSequencerError>;
}

impl Storage {
	pub fn try_new(path: &str) -> Result<Self, DaSequencerError> {
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
			Ok(BlockHeight(last_height + 1))
		} else {
			Ok(BlockHeight(1))
		}
	}

	fn notify_block_celestia_sent(&self, heigh: BlockHeight) -> Result<(), DaSequencerError> {
		todo!();
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
				let block: SequencerBlock = bcs::from_bytes(&bytes)
					.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;
				Ok(Some(block))
			}
			None => Ok(None),
		}
	}

	fn get_block_with_digest(
		&self,
		id: SequencerBlockDigest,
	) -> Result<Option<SequencerBlock>, DaSequencerError> {
		let cf = self.db.cf_handle(cf::BLOCKS_BY_DIGEST).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks_by_digest".into())
		})?;

		let key = id.0;

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
			DaSequencerError::StorageAccess("Missing colomn family: pending transactions".into())
		})?;

		// Collect and deserialize all pending transactions
		let mut transactions: Vec<Transaction> = Vec::new();

		let iter = self.db.iterator_cf(&cf_pending, rocksdb::IteratorMode::Start);
		for item in iter {
			let (_key, value) = item.map_err(|e| DaSequencerError::RocksDbError(e.to_string()))?;
			let tx: Transaction = bcs::from_bytes(&value)
				.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;
			transactions.push(tx);
		}

		if transactions.is_empty() {
			return Ok(None);
		}

		// Sort transactions using their Ord implementation
		transactions.sort();

		let mut selected_txs = Vec::new();
		let mut block_size: u64 = 0;

		for tx in &transactions {
			let tx_size = bcs::to_bytes(tx)
				.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?
				.len() as u64; // The size of the tx is the len of the bytes Vec<u8>

			if block_size + tx_size > MAX_SEQUENCER_BLOCK_SIZE {
				break;
			}

			block_size += tx_size;
			selected_txs.push(tx.clone());
		}

		// Notes that this method will never return less that 1
		let height: u64 = self.determine_next_block_height()?.into();

		let parent_height = BlockHeight(height - 1);
		let parent_digest = self
			.get_block_at_height(parent_height)?
			.ok_or_else(|| DaSequencerError::StorageFormat("Missing parent block".into()))?
			.get_block_digest();

		// Build the block
		let tx_set: BTreeSet<_> = selected_txs.clone().into_iter().collect();
		let block = Block::new(BlockMetadata::default(), Id::new(parent_digest.0), tx_set);
		let sequencer_block = SequencerBlock::try_new(self.determine_next_block_height()?, block)?;

		let block_bytes = bcs::to_bytes(&sequencer_block)
			.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;

		let cf_blocks = self.db.cf_handle(cf::BLOCKS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks".into())
		})?;
		let cf_digests = self.db.cf_handle(cf::BLOCKS_BY_DIGEST).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks_by_digest".into())
		})?;

		let mut write_batch = WriteBatch::default();
		let height_key = height.to_be_bytes();

		write_batch.put_cf(&cf_blocks, height_key, &block_bytes);
		write_batch.put_cf(&cf_digests, sequencer_block.get_block_digest().0, &height_key);

		// Remove the selected transactions from pending
		for tx in &selected_txs {
			write_batch.delete_cf(&cf_pending, tx.id());
		}

		self.db
			.write(write_batch)
			.map_err(|e| DaSequencerError::RocksDbError(e.to_string()))?;

		Ok(Some(sequencer_block))
	}

	fn get_celestia_height_for_block(
		&self,
		height: BlockHeight,
	) -> Result<Option<CelestiaHeight>, DaSequencerError> {
		todo!();
	}

	fn set_block_celestia_height(
		&self,
		block_heigh: BlockHeight,
		celestia_heigh: CelestiaHeight,
	) -> Result<(), DaSequencerError> {
		todo!()
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
	fn test_get_block_at_height_returns_correct_block() {
		use crate::block::{BlockHeight, SequencerBlock};
		use movement_types::block::Block;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		let block_height = BlockHeight(42);
		let dummy_block = Block::default();
		let sequencer_block = SequencerBlock { height: block_height, block: dummy_block.clone() };

		let encoded_block =
			bcs::to_bytes(&sequencer_block).expect("failed to serialize SequencerBlock");

		let cf = storage.db.cf_handle(cf::BLOCKS).expect("missing 'blocks' column family");
		let key = block_height.0.to_be_bytes();
		storage.db.put_cf(&cf, key, encoded_block).expect("failed to write to db");

		let result = storage.get_block_at_height(block_height).expect("get_block_at_height failed");

		assert!(result.is_some(), "expected Some(block), got None");
		let fetched_block = result.unwrap();
		assert_eq!(fetched_block.height, sequencer_block.height);
		assert_eq!(fetched_block.block, sequencer_block.block);
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
		let sequencer_block = SequencerBlock { height: block_height, block: dummy_block.clone() };
		let digest = sequencer_block.get_block_digest();

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
		storage
			.db
			.put_cf(&cf_digests, digest.0, key)
			.expect("failed to write digest mapping");

		let result = storage.get_block_with_digest(digest).expect("get_block_with_digest failed");

		assert!(result.is_some(), "Expected Some(block), got None");
		let fetched_block = result.unwrap();
		assert_eq!(fetched_block.height, sequencer_block.height);
		assert_eq!(fetched_block.block, sequencer_block.block);

		let stored_height_bytes = storage
			.db
			.get_cf(&cf_digests, digest.0)
			.expect("failed to read digest mapping")
			.expect("digest not found");
		assert_eq!(stored_height_bytes, key);
	}

	#[test]
	fn test_get_block_with_digest_returns_none_for_unknown_digest() {
		use crate::block::SequencerBlockDigest;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		let fake_digest = SequencerBlockDigest([0u8; 32]);
		let result = storage.get_block_with_digest(fake_digest);

		assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result);
		assert!(result.unwrap().is_none(), "Expected None for unknown digest, got Some");
	}

	#[test]
	#[ignore]
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
		let transactions: Vec<_> = block.block.transactions().cloned().collect();
		assert_eq!(transactions.len(), 1, "Expected 1 transaction in block");
		assert_eq!(transactions[0], tx_assert, "Transaction in block does not match original");

		let cf_pending = storage
			.db
			.cf_handle(cf::PENDING_TRANSACTIONS)
			.expect("missing 'pending_transactions' CF");
		let maybe_tx = storage.db.get_cf(&cf_pending, tx_id).expect("failed to read pending tx");
		assert!(maybe_tx.is_none(), "Pending transaction was not cleared");

		let cf_blocks = storage.db.cf_handle(cf::BLOCKS).expect("missing 'blocks' CF");
		let height_key = block.height.0.to_be_bytes();
		let stored_bytes = storage
			.db
			.get_cf(&cf_blocks, height_key)
			.expect("failed to read stored block")
			.expect("block not found");

		let stored_block: SequencerBlock =
			bcs::from_bytes(&stored_bytes).expect("failed to deserialize stored block");
		assert_eq!(stored_block, block, "Stored block does not match produced block");
	}
}
