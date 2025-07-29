use crate::celestia::CelestiaBlob;
use crate::{
	block::BlockHeight, celestia::CelestiaHeight, error::DaSequencerError, DaSequencerExternalDa,
};
use movement_types::block;
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
		_block: block::Id,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send {
		futures::future::ready(Ok(()))
	}

	fn get_blob_at_height(
		&self,
		_height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<CelestiaBlob>, DaSequencerError>> + Send {
		//TODO return dummy error for now.
		futures::future::ready(Err(DaSequencerError::DeserializationFailure))
	}

	fn bootstrap(
		&self,
		_current_block_height: BlockHeight,
		_last_finalized_celestia_height: Option<CelestiaHeight>,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send {
		futures::future::ready(Ok(()))
	}
}
