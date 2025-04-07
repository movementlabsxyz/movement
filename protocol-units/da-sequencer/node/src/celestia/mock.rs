use super::{blob::Blob, CelestiaHeight};
use crate::{
	block::{BlockHeight, SequencerBlockDigest},
	error::DaSequencerError,
	DaSequencerExternalDa,
};
use std::future::Future;

#[derive(Debug, Clone)]
pub struct CelestiaMock {}

impl CelestiaMock {
	pub fn new() -> Self {
		CelestiaMock {}
	}
}

impl DaSequencerExternalDa for CelestiaMock {
	fn send_block(
		&self,
		block: SequencerBlockDigest,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send {
		futures::future::ready(Ok(()))
	}

	fn get_blobs_at_height(
		&self,
		height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<Vec<Blob>>, DaSequencerError>> + Send {
		//TODO return dummy error for now.
		futures::future::ready(Err(DaSequencerError::DeserializationFailure))
	}

	fn bootstrap(
		&self,
		current_block_height: BlockHeight,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send {
		futures::future::ready(Ok(()))
	}
}
