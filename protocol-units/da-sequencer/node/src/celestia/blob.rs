use crate::block::SequencerBlockDigest;

#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Blob(pub Vec<SequencerBlockDigest>);
