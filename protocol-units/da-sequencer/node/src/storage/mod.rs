use crate::{
	batch::{DaBatch, FullNodeTx},
	block::{BlockHeight, SequencerBlock, SequencerBlockDigest},
	celestia::CelestiaHeight,
	error::DaSequencerError,
};
use bincode;
use rocksdb::{ColumnFamilyDescriptor, Options, WriteBatch, DB};
use std::{fmt, result::Result, sync::Arc};

pub mod cf {
	pub const PENDING_TRANSACTIONS: &str = "pending_transactions";
	pub const BLOCKS: &str = "blocks";
}

pub struct Storage {
	db: Arc<DB>,
}

impl Storage {
	pub fn try_new(path: &str) -> Result<Self, DaSequencerError> {
		let mut options = Options::default();

		options.create_if_missing(true);
		options.create_missing_column_families(true);

		let pending_transactions_cf =
			ColumnFamilyDescriptor::new(cf::PENDING_TRANSACTIONS, Options::default());
		let blocks_cf = ColumnFamilyDescriptor::new(cf::BLOCKS, Options::default());

		let db = DB::open_cf_descriptors(&options, path, [pending_transactions_cf, blocks_cf])
			.map_err(|e| DaSequencerError::Generic(e.to_string()))?;

		Ok(Storage { db: Arc::new(db) })
	}

	/// Save all batch's Tx in the pending Tx table. The batch's Tx has been verified and validated.
	pub fn write_batch(
		&self,
		batch: DaBatch<FullNodeTx>,
	) -> std::result::Result<(), DaSequencerError> {
		let cf = self.db.cf_handle(cf::PENDING_TRANSACTIONS).ok_or_else(|| {
			DaSequencerError::Generic("Missing column family: pending_transactions".into())
		})?;
		let tx = batch.data;
		let key = tx.id.0;
		let value = bincode::serialize(&tx)
			.map_err(|e| DaSequencerError::Generic(format!("Serialization error: {}", e)))?;

		let mut write_batch = WriteBatch::default();
		write_batch.put_cf(&cf, key, value);

		self.db
			.write(write_batch)
			.map_err(|e| DaSequencerError::Generic(format!("DB write error: {}", e)))?;

		Ok(())
	}

	/// Return, if exists, the Block at the given height.
	pub fn get_block_at_height(
		&self,
		height: BlockHeight,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		let cf = self
			.db
			.cf_handle(cf::BLOCKS)
			.ok_or_else(|| DaSequencerError::Generic("Missing column family: blocks".into()))?;

		let key: [u8; 8] = height.0.to_be_bytes();

		match self.db.get_cf(&cf, key).map_err(|e| DaSequencerError::Generic(e.to_string()))? {
			Some(bytes) => {
				let block: SequencerBlock = bincode::deserialize(&bytes).map_err(|e| {
					DaSequencerError::Generic(format!("Deserialization error: {}", e))
				})?;
				Ok(Some(block))
			}
			None => Ok(None),
		}
	}

	/// Return, if exists, the Block with specified sequencer id.
	pub fn get_block_with_digest(
		&self,
		id: SequencerBlockDigest,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		todo!();
	}

	/// Produce next block with pending Tx.
	/// Generate the new height.
	/// Aggregate all pending Tx until the block is filled.
	/// A block is filled if no more Tx are pending or it's size is more than the max size.
	/// All pending Tx added to the block are removed from the pending Tx table.
	/// Save the block for this height
	/// Return the block.
	pub fn produce_next_block(
		&self,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		todo!();
	}

	/// Return, if exists, the Celestia height for given block height.
	pub fn get_celestia_height_for_block(
		&self,
		heigh: BlockHeight,
	) -> std::result::Result<Option<CelestiaHeight>, DaSequencerError> {
		todo!();
	}

	/// Notify that the block at the given height has been sent to Celestia.

	pub fn notify_block_celestia_sent(
		&self,
		heigh: BlockHeight,
	) -> std::result::Result<(), DaSequencerError> {
		todo!();
	}

	/// Set the Celestia height for a given block height.
	pub fn set_block_celestia_height(
		&self,
		block_heigh: BlockHeight,
		celestia_heigh: CelestiaHeight,
	) -> std::result::Result<(), DaSequencerError> {
		todo!()
	}
}

impl fmt::Debug for Storage {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// can't print the actual DB content, but indicate it's present for now ..
		f.debug_struct("Storage")
			.field("db", &"RocksDB<Arc>") //
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use aptos_crypto::ed25519::Ed25519PublicKey;
	use tempfile::TempDir;

	#[test]
	fn test_try_new_creates_storage_successfully() {
		// Create a temporary directory for the RocksDB instance
		let temp_dir = TempDir::new().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();

		// Attempt to create the Storage
		let storage = Storage::try_new(path);

		// Assert it's Ok and the inner DB is accessible
		assert!(storage.is_ok(), "Expected Ok(Storage), got Err: {:?}", storage);
	}

	#[test]
	fn test_try_new_invalid_path_should_fail() {
		// Try to open a DB at an invalid path
		// Using an empty string usually results in an error
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

		// Create a temporary DB
		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		// Create a dummy transaction
		let tx = Transaction::test_only_new(
			b"test data".to_vec(),
			1,   // application_priority
			123, // sequence_number
		);
		let tx_id = tx.id; // capture the ID before tx is moved

		let batch = DaBatch::test_only_new(tx);

		// Call write_batch
		storage.write_batch(batch).expect("write_batch failed");

		// Read back manually from RocksDB
		let cf = storage
			.db
			.cf_handle(cf::PENDING_TRANSACTIONS)
			.expect("missing pending_transactions CF");

		let key = tx_id.0;
		let stored_bytes =
			storage.db.get_cf(&cf, key).expect("read failed").expect("no data found");

		// Deserialize and assert
		let stored_tx: Transaction =
			bincode::deserialize(&stored_bytes).expect("failed to deserialize stored transaction");

		assert_eq!(stored_tx.id, tx_id);
		assert_eq!(stored_tx.sequence_number(), 123);
		assert_eq!(stored_tx.application_priority(), 1);
		assert_eq!(stored_tx.data(), b"test data");
	}

	#[test]
	fn test_get_block_at_height_returns_correct_block() {
		use crate::block::{BlockHeight, SequencerBlock};
		use bincode;
		use movement_types::block::Block;
		use tempfile::tempdir;

		// Setup: create a temporary DB
		let temp_dir = tempdir().expect("failed to create temp dir");
		let path = temp_dir.path().to_str().unwrap();
		let storage = Storage::try_new(path).expect("failed to create storage");

		// Create dummy block data
		let block_height = BlockHeight(42);
		let dummy_block = Block::default();
		let sequencer_block = SequencerBlock { height: block_height, block: dummy_block.clone() };

		// Serialize the block with bincode v1
		let encoded_block =
			bincode::serialize(&sequencer_block).expect("failed to serialize SequencerBlock");

		// Insert into RocksDB manually
		let cf = storage.db.cf_handle(cf::BLOCKS).expect("missing 'blocks' column family");

		let key = block_height.0.to_be_bytes();
		storage.db.put_cf(&cf, key, encoded_block).expect("failed to write to db");

		// Test the method
		let result = storage.get_block_at_height(block_height).expect("get_block_at_height failed");

		assert!(result.is_some(), "expected Some(block), got None");
		let fetched_block = result.unwrap();
		assert_eq!(fetched_block.height, sequencer_block.height);
		assert_eq!(fetched_block.block, sequencer_block.block);
	}
}
