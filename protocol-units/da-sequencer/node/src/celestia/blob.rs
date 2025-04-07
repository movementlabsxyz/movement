use crate::block::{BlockHeight, SequencerBlockDigest};
use std::slice::Iter;

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct Blob(pub Vec<SequencerBlockDigest>);

impl Blob {
	pub fn iter(&self) -> Iter<'_, SequencerBlockDigest> {
		self.0.iter()
	}

	pub fn last_block_height(&self) -> Option<BlockHeight> {
		self.0.last().map(|b| b.height)
	}
}

impl IntoIterator for Blob {
	type Item = SequencerBlockDigest;
	type IntoIter = <Vec<SequencerBlockDigest> as IntoIterator>::IntoIter;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}
