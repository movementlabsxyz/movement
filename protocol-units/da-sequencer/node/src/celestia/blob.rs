use movement_types::block;
use serde::{Deserialize, Serialize};
use std::slice::Iter;

/// The blob format that is stored in Celestia DA.
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CelestiaBlob(Vec<block::Id>);

impl CelestiaBlob {
	pub fn iter(&self) -> Iter<'_, block::Id> {
		self.0.iter()
	}

	pub fn last_block_id(&self) -> Option<block::Id> {
		self.0.last().copied()
	}

	pub fn to_vec(self) -> Vec<block::Id> {
		self.0
	}
}

impl IntoIterator for CelestiaBlob {
	type Item = block::Id;
	type IntoIter = <Vec<block::Id> as IntoIterator>::IntoIter;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl From<Vec<block::Id>> for CelestiaBlob {
	fn from(value: Vec<block::Id>) -> Self {
		Self(value)
	}
}
