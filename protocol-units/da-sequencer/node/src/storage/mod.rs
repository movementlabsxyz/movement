use crate::{
	batch::{DaBatch, FullNodeTx},
	block::{BlockHeight, SequencerBlock, SequencerBlockDigest},
	celestia::CelestiaHeight,
	error::DaSequencerError,
};
use bcs;
use movement_types::block::{Block, BlockMetadata, Id};
use movement_types::transaction::Transaction;
use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};
use std::{collections::BTreeSet, result::Result, sync::Arc};

pub mod cf {
	pub const PENDING_TRANSACTIONS: &str = "pending_transactions";
	pub const BLOCKS: &str = "blocks";
	pub const BLOCKS_BY_DIGEST: &str = "blocks_by_digest";
}

#[derive(Debug)]
pub struct Storage {
	db: Arc<DB>,
}

pub trait DaSequencerStorage {
	/// Save all batch's Tx in the pending Tx table. The batch's Tx has been verified and validated.
	fn write_batch(&self, batch: DaBatch<FullNodeTx>) -> std::result::Result<(), DaSequencerError>;

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

	/// Produce next block with pending Tx.
	/// Generate the new height.
	/// Aggregate all pending Tx until the block is filled.
	/// A block is filled if no more Tx are pending or it's size is more than the max size.
	/// All pending Tx added to the block are removed from the pending Tx table.
	/// Save the block for this height
	/// Return the block.
	fn produce_next_block(&self) -> std::result::Result<Option<SequencerBlock>, DaSequencerError>;

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
		.map_err(|e| DaSequencerError::Generic(e.to_string()))?;

		Ok(Storage { db: Arc::new(db) })
	}

	fn determine_next_block_height(&self) -> Result<BlockHeight, DaSequencerError> {
		let cf = self.db.cf_handle(cf::BLOCKS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks".into())
		})?;

		let mut iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::End);
		if let Some(Ok((key, _value))) = iter.next() {
			if key.len() != 8 {
				return Err(DaSequencerError::Generic("Invalid block height key length".into()));
			}

			let mut arr = [0u8; 8];
			arr.copy_from_slice(&key);
			let last_height = u64::from_be_bytes(arr);
			Ok(BlockHeight(last_height + 1))
		} else {
			Ok(BlockHeight(0))
		}
	}

	fn notify_block_celestia_sent(&self, heigh: BlockHeight) -> Result<(), DaSequencerError> {
		todo!();
	}
}

impl DaSequencerStorage for Storage {
	fn write_batch(&self, batch: DaBatch<FullNodeTx>) -> Result<(), DaSequencerError> {
		let cf = self.db.cf_handle(cf::PENDING_TRANSACTIONS).ok_or_else(|| {
			DaSequencerError::Generic("Missing column family: pending_transactions".into())
		})?;
		let tx = batch.data();
		let key = tx.id.0;
		let value = bcs::to_bytes(tx)
			.map_err(|e| DaSequencerError::Generic(format!("Serialization error: {}", e)))?;

		let mut write_batch = WriteBatch::default();
		write_batch.put_cf(&cf, key, value);

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
		let next_height = self.determine_next_block_height()?;

		let cf_pending = self.db.cf_handle(cf::PENDING_TRANSACTIONS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: pending_transactions".into())
		})?;

		let mut txs = Vec::new();
		let iter = self.db.iterator_cf(&cf_pending, rocksdb::IteratorMode::Start);
		let mut batch = WriteBatch::default();

		for item in iter {
			let (key, value) = item.map_err(|e| DaSequencerError::Generic(e.to_string()))?;

			let tx: FullNodeTx = bcs::from_bytes(&value)
				.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;

			txs.push(tx);
			batch.delete_cf(&cf_pending, &key);
		}

		if txs.is_empty() {
			return Ok(None);
		}

		let tx_set: BTreeSet<Transaction> = txs.into_iter().collect();

		let block = Block::new(BlockMetadata::default(), Id::default(), tx_set);

		let sequencer_block = SequencerBlock::try_new(next_height, block)?;

		let cf_blocks = self.db.cf_handle(cf::BLOCKS).ok_or_else(|| {
			DaSequencerError::StorageAccess("Missing column family: blocks".into())
		})?;

		let encoded = bcs::to_bytes(&sequencer_block)
			.map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;

		let height_key = next_height.0.to_be_bytes();
		batch.put_cf(&cf_blocks, height_key, encoded);

		self.db
			.write(batch)
			.map_err(|e| DaSequencerError::RocksDbError(e.to_string()))?;

		Ok(Some(sequencer_block))
	}

	fn get_celestia_height_for_block(
		&self,
		heigh: BlockHeight,
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
		match result {
			Err(DaSequencerError::Generic(msg)) => {
				assert!(
					msg.contains("Invalid argument") || msg.contains("No such file"),
					"Unexpected error message: {}",
					msg
				);
			}
			_ => panic!("Expected Generic error variant"),
		}
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
		let tx_id = tx.id;
		let batch = DaBatch::test_only_new(tx);

		storage.write_batch(batch).expect("write_batch failed");

		let cf = storage
			.db
			.cf_handle(cf::PENDING_TRANSACTIONS)
			.expect("missing pending_transactions CF");

		let key = tx_id.0;
		let stored_bytes =
			storage.db.get_cf(&cf, key).expect("read failed").expect("no data found");

		let stored_tx: Transaction =
			bcs::from_bytes(&stored_bytes).expect("failed to deserialize stored transaction");

		assert_eq!(stored_tx.id, tx_id);
		assert_eq!(stored_tx.sequence_number(), 123);
		assert_eq!(stored_tx.application_priority(), 1);
		assert_eq!(stored_tx.data(), b"test data");
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
	fn test_produce_next_block_generates_block_and_clears_pending_tx() {
		use crate::batch::DaBatch;
		use movement_types::transaction::Transaction;
		use tempfile::tempdir;

		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		let tx = Transaction::test_only_new(b"test data".to_vec(), 0, 1);
		let tx_id = tx.id;
		let batch = DaBatch::test_only_new(tx.clone());
		storage.write_batch(batch).expect("failed to write batch");

		let maybe_block = storage.produce_next_block().expect("produce_next_block failed");

		assert!(maybe_block.is_some(), "Expected Some(block), got None");
		let block = maybe_block.unwrap();
		let transactions: Vec<_> = block.block.transactions().cloned().collect();
		assert_eq!(transactions.len(), 1, "Expected 1 transaction in block");
		assert_eq!(transactions[0], tx, "Transaction in block does not match original");

		let cf_pending = storage
			.db
			.cf_handle(cf::PENDING_TRANSACTIONS)
			.expect("missing 'pending_transactions' CF");
		let maybe_tx = storage.db.get_cf(&cf_pending, tx_id.0).expect("failed to read pending tx");
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
