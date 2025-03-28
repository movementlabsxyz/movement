use crate::batch::DaBatch;
use crate::batch::FullNodeTxs;
use crate::block::BlockHeight;
use crate::block::SequencerBlock;
use crate::block::SequencerBlockDigest;
use crate::celestia::CelestiaHeight;
use crate::error::DaSequencerError;

pub trait DaSequencerStorage {
	/// Save all batch's Tx in the pending Tx table. The batch's Tx has been verified and validated.
	fn write_batch(&self, batch: DaBatch<FullNodeTxs>) -> std::result::Result<(), DaSequencerError>;

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

pub struct Storage {}

impl Storage {
	/// Save all batch's Tx in the pending Tx table. The batch's Tx has been verified and validated.
	pub fn write_batch(
		&self,
		batch: DaBatch<FullNodeTxs>,
	) -> std::result::Result<(), DaSequencerError> {
		todo!();
	}

	/// Return, if exists, the Block at the given height.
	pub fn get_block_at_height(
		&self,
		height: BlockHeight,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		todo!();
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
		height: BlockHeight,
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
