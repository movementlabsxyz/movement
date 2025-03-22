use movement_types::block::Block;

#[derive(Debug, Clone)]
pub struct SequencerBlockDigest(Vec<u8>);

/// The id for an Ir Blob
impl SequencerBlockDigest {
	pub fn new(id: Vec<u8>) -> Self {
		SequencerBlockDigest(id)
	}

	pub fn as_slice(&self) -> &[u8] {
		self.0.as_slice()
	}

	pub fn into_vec(self) -> Vec<u8> {
		self.0
	}
}

impl From<Vec<u8>> for SequencerBlockDigest {
	fn from(id: Vec<u8>) -> Self {
		SequencerBlockDigest(id)
	}
}

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockHeight(pub u64);

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SequencerBlock {
	height: BlockHeight,
	block: Block,
}

impl SequencerBlock {
	pub fn get_block_digest(&self) -> SequencerBlockDigest {
		SequencerBlockDigest(self.block.id().to_vec())
	}
}
