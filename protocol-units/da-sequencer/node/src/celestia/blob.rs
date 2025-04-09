use crate::block::SequencerBlockDigest;

use serde::{Deserialize, Serialize};

/// The blob format that is stored in Celestia DA.
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CelestiaBlobData {
	pub digests: Vec<SequencerBlockDigest>,
}
