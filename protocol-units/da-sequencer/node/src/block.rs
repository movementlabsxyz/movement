use movement_types::block::{self, Block, Transactions};
use serde::{Deserialize, Serialize};
use std::ops::Add;

use crate::error::DaSequencerError;

// TODO: use a sensible value for the max sequencer block size
pub const MAX_SEQUENCER_BLOCK_SIZE: u64 = 1_000_000; // 1 MB

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

// Rust interprets (small) integer literals without a type suffix as i32
impl<T: Into<i64>> Add<T> for BlockHeight {
	type Output = Self;

	fn add(self, rhs: T) -> Self::Output {
		let value = <i64 as TryInto<u64>>::try_into(rhs.into()).expect("Added a negative value");
		BlockHeight(self.0 + value)
	}
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequencerBlock {
	height: BlockHeight,
	block: Block,
}

impl SequencerBlock {
	/// Try to construct a SequencerBlock, but fail if it exceeds the max encoded size.
	pub fn try_new(height: BlockHeight, block: Block) -> Result<Self, DaSequencerError> {
		let sb = SequencerBlock { height, block };
		Ok(sb)
	}

	pub fn id(&self) -> block::Id {
		self.block.id()
	}

	pub fn height(&self) -> BlockHeight {
		self.height
	}

	pub fn transactions(&self) -> Transactions {
		self.block.transactions()
	}

	pub fn len(&self) -> usize {
		self.block.transactions().count()
	}

	pub fn inner_block(self) -> Block {
		self.block
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
