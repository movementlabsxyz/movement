use movement_types::block::Block;
use serde::{Deserialize, Serialize};

use crate::error::DaSequencerError;

// TODO: use a sensible value for the max sequencer block size
const MAX_SEQUENCER_BLOCK_SIZE: u64 = 1_000_000; // 1 MB

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequencerBlockDigest(pub [u8; 32]);

/// The id for an Ir Blob
impl SequencerBlockDigest {
	pub fn new(id: [u8; 32]) -> Self {
		SequencerBlockDigest(id)
	}

	pub fn as_slice(&self) -> &[u8] {
		self.0.as_slice()
	}

	pub fn into_vec(&self) -> Vec<u8> {
		self.0.to_vec()
	}
}

#[derive(
	Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct BlockHeight(pub u64);

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequencerBlock {
	pub height: BlockHeight,
	pub block: Block,
}

impl SequencerBlock {
	/// Try to construct a SequencerBlock, but fail if it exceeds the max encoded size.
	pub fn try_new(height: BlockHeight, block: Block) -> Result<Self, DaSequencerError> {
		todo!()
	}

	pub fn get_block_digest(&self) -> SequencerBlockDigest {
		SequencerBlockDigest(*self.block.id().as_bytes())
	}

	pub fn validate_size(&self) -> Result<(), DaSequencerError> {
		let bytes =
			bcs::to_bytes(self).map_err(|e| DaSequencerError::Deserialization(e.to_string()))?;
		let size = bytes.len() as u64;

		if size > MAX_SEQUENCER_BLOCK_SIZE {
			Err(DaSequencerError::SizeExceedsMax(MAX_SEQUENCER_BLOCK_SIZE as usize))
		} else {
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {

	#[test]
	#[ignore]
	//TODO: will be dealt with PR #1159
	fn test_sequencer_block_rejects_block_larger_than_max_size() {
		use super::MAX_SEQUENCER_BLOCK_SIZE;
		use crate::block::{BlockHeight, SequencerBlock};
		use crate::DaSequencerError;
		use movement_types::{block::Block, transaction::Transaction};
		use std::collections::BTreeSet;

		// Fill enough transactions with large data to exceed MAX_SEQUENCER_BLOCK_SIZE (1MB)
		let mut transactions = BTreeSet::new();
		let mut total_size = 0;

		while total_size < MAX_SEQUENCER_BLOCK_SIZE as usize + 100_000 {
			let data = vec![0u8; 100_000]; // 100 KB each
			let tx = Transaction::test_only_new(data, 0, total_size as u64);
			total_size += 100_000;
			transactions.insert(tx);
		}

		let block = Block::new(Default::default(), Default::default(), transactions);

		let result = SequencerBlock::try_new(BlockHeight(0), block);

		assert!(
			matches!(&result, Err(DaSequencerError::SizeExceedsMax(size)) if *size == MAX_SEQUENCER_BLOCK_SIZE as usize),
			"Expected error for oversized block, got: {:?}",
			result
		)
	}
}
