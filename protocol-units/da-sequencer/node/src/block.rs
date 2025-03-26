use movement_types::block::Block;
use serde::{Deserialize, Serialize};

use crate::error::DaSequencerError;

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
		let sb = SequencerBlock { height, block };
		sb.validate_size()?;
		Ok(sb)
	}

	pub fn get_block_digest(&self) -> SequencerBlockDigest {
		SequencerBlockDigest(*self.block.id().as_bytes())
	}

	pub fn validate_size(&self) -> Result<(), DaSequencerError> {
		let size = bincode::serialized_size(self)
			.map_err(|e| DaSequencerError::Generic(format!("Serialization failed: {}", e)))?;

		if size > MAX_SEQUENCER_BLOCK_SIZE {
			Err(DaSequencerError::Generic(format!(
				"SequencerBlock exceeds max size: {} > {} bytes",
				size, MAX_SEQUENCER_BLOCK_SIZE
			)))
		} else {
			Ok(())
		}
	}
}
