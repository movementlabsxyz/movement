use crate::block::{BlockHeight, SequencerBlockDigest};
use std::slice::Iter;

use serde::{Deserialize, Serialize};

/// The blob format that is stored in Celestia DA.
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CelestiaBlobData {
	pub digests: Vec<SequencerBlockDigest>,
}

impl CelestiaBlobData {
	pub fn iter(&self) -> Iter<'_, SequencerBlockDigest> {
		self.digests.iter()
	}

	pub fn last_block_height(&self) -> Option<BlockHeight> {
		self.digests.last().map(|b| b.height)
	}
}

impl IntoIterator for CelestiaBlobData {
	type Item = SequencerBlockDigest;
	type IntoIter = <Vec<SequencerBlockDigest> as IntoIterator>::IntoIter;

	fn into_iter(self) -> Self::IntoIter {
		self.digests.into_iter()
	}
}
