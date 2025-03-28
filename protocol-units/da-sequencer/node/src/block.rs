use movement_types::block::Block;

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

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockHeight(pub u64);

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequencerBlock {
	height: BlockHeight,
	block: Block,
}

impl SequencerBlock {
	pub fn get_block_digest(&self) -> SequencerBlockDigest {
		SequencerBlockDigest(*self.block.id().as_bytes())
	}

	pub fn get_height(&self) -> BlockHeight {
		self.height
	}
}
