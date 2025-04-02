use movement_types::block::Block;
use serde::{Deserialize, Serialize};

use crate::error::DaSequencerError;

// TODO: use a sensible value for the max sequencer block size
pub const MAX_SEQUENCER_BLOCK_SIZE: u64 = 1_000_000; // 1 MB

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequencerBlockDigest(pub [u8; 32]);

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
	fn from(v: BlockHeight) -> Self {
		v.0
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
		Ok(SequencerBlock { height, block })
	}

	pub fn get_block_digest(&self) -> SequencerBlockDigest {
		SequencerBlockDigest(*self.block.id().as_bytes())
	}
}
