use crate::batch::DaBatch;
use crate::batch::FullnodeTx;
use crate::block::BlockHeight;
use crate::block::SequencerBlock;
use crate::block::SequencerBlockDigest;
use crate::celestia::CelestiaHeight;
use crate::error::DaSequencerError;
use bincode;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::fmt;
use std::sync::Arc;

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
		batch: DaBatch<FullnodeTx>,
	) -> std::result::Result<(), DaSequencerError> {
		todo!();
	}

	/// Return, if exists, the Block at the given height.
	pub fn get_block_at_height(
		&self,
		height: BlockHeight,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		let cf = self
			.db
			.cf_handle(cf::BLOCKS)
			.ok_or_else(|| DaSequencerError::Generic("Missing column family: blocks".into()));

		let key: [u8; 8] = height.0.to_be_bytes();

		match self.db.get_cf(cf, key)? {
			Some(bytes) => {
				let (block, _) = bincode::decode_from_slice::<SequencerBlock, _>(
					&bytes,
					bincode::config::standard(),
				)
				.map_err(|e| DaSequencerError::Generic(format!("Deserialization error: {}", e)))?;
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
}
