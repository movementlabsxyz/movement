use movement_types::block::Block;
use serde::{Deserialize, Serialize};

use crate::error::DaSequencerError;

// TODO: use a sensible value for the max sequencer block size
pub const MAX_SEQUENCER_BLOCK_SIZE: u64 = 1_000_000; // 1 MB

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SequencerBlockDigest {
	pub height: BlockHeight,
	pub id: [u8; Self::DIGEST_SIZE],
}

impl SequencerBlockDigest {
	/// Size of a digest in bytes.
	pub const DIGEST_SIZE: usize = 32;

	pub fn new(height: BlockHeight, id: [u8; 32]) -> Self {
		SequencerBlockDigest { height, id }
	}
}

#[derive(
	Serialize, Deserialize, Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct BlockHeight(pub u64);

impl BlockHeight {
	/// Returns the parent block height.
	/// If this is the genesis block (height 0), returns 0.
	pub fn parent(&self) -> BlockHeight {
		BlockHeight(self.0.saturating_sub(1))
	}
}

impl From<u64> for BlockHeight {
	fn from(value: u64) -> Self {
		BlockHeight(value)
	}
}

impl From<BlockHeight> for u64 {
	fn from(height: BlockHeight) -> Self {
		height.0
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequencerBlock {
	pub height: BlockHeight,
	pub block: Block,
}

impl SequencerBlock {
	/// Try to construct a SequencerBlock, but fail if it exceeds the max encoded size.
	pub fn try_new(height: BlockHeight, block: Block) -> Result<Self, DaSequencerError> {
		let sb = SequencerBlock { height, block };
		Ok(sb)
	}

	pub fn get_block_digest(&self) -> SequencerBlockDigest {
		SequencerBlockDigest::new(self.height, *self.block.id().as_bytes())
	}
}

impl TryFrom<&[u8]> for SequencerBlock {
	type Error = DaSequencerError;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		bcs::from_bytes(bytes).map_err(|e| DaSequencerError::Deserialization(e.to_string()))
	}
}

impl TryInto<Vec<u8>> for SequencerBlock {
	type Error = DaSequencerError;

	fn try_into(self) -> Result<Vec<u8>, Self::Error> {
		(&self).try_into()
	}
}

impl TryInto<Vec<u8>> for &SequencerBlock {
	type Error = DaSequencerError;

	fn try_into(self) -> Result<Vec<u8>, Self::Error> {
		bcs::to_bytes(self).map_err(|e| DaSequencerError::Deserialization(e.to_string()))
	}
}
