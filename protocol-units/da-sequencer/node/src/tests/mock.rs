use crate::batch::DaBatch;
use crate::batch::FullNodeTxs;
use crate::block::BlockHeight;
use crate::block::SequencerBlockDigest;
use crate::celestia::blob::Blob;
use crate::celestia::CelestiaHeight;
use crate::DaSequencerError;
use crate::DaSequencerExternalDa;
use crate::DaSequencerStorage;
use crate::SequencerBlock;
use std::cell::RefCell;
use std::future::Future;

#[derive(Debug, Clone)]
pub struct StorageMock {
	pub batches: RefCell<Vec<DaBatch<FullNodeTxs>>>,
	pub current_height: u64,
}

impl StorageMock {
	pub fn new() -> Self {
		StorageMock { batches: RefCell::new(Vec::new()), current_height: 0 }
	}
}

impl DaSequencerStorage for StorageMock {
	fn write_batch(
		&self,
		batch: DaBatch<FullNodeTxs>,
	) -> std::result::Result<(), DaSequencerError> {
		tracing::info!("Mock: Storage, call write_batch");
		self.batches.borrow_mut().push(batch);
		Ok(())
	}

	fn get_block_at_height(
		&self,
		height: BlockHeight,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		todo!();
	}

	fn get_block_with_digest(
		&self,
		id: SequencerBlockDigest,
	) -> std::result::Result<Option<SequencerBlock>, DaSequencerError> {
		todo!();
	}

	fn produce_next_block(&self) -> Result<Option<SequencerBlock>, DaSequencerError> {
		Ok(None)
	}

	fn get_celestia_height_for_block(
		&self,
		heigh: BlockHeight,
	) -> std::result::Result<Option<CelestiaHeight>, DaSequencerError> {
		todo!();
	}

	fn set_block_celestia_height(
		&self,
		block_heigh: BlockHeight,
		celestia_heigh: CelestiaHeight,
	) -> std::result::Result<(), DaSequencerError> {
		todo!();
	}
}

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
		_block: SequencerBlockDigest,
	) -> impl Future<Output = Result<(), DaSequencerError>> + Send {
		futures::future::ready(Ok(()))
	}

	fn get_blob_at_height(
		&self,
		_height: CelestiaHeight,
	) -> impl Future<Output = Result<Option<Blob>, DaSequencerError>> + Send {
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
