use mempool_util::{MempoolBlockOperationsError, MempoolTransactionOperationsError};
use movement_types::{AtomicTransactionBundle, Block, Transaction};
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SequencerError {
	#[error("MempoolTransactionOperationsError error: {0}")]
	MempoolTransactionOperationsError(#[from] MempoolTransactionOperationsError),
	#[error("MempoolBlockOperationsError error: {0}")]
	MempoolBlockOperationsError(#[from] MempoolBlockOperationsError),
}

pub type SequencerResult<T> = Result<T, SequencerError>;

#[allow(async_fn_in_trait)]
pub trait Sequencer {
	async fn publish(&self, atb: Transaction) -> SequencerResult<()>;

	async fn wait_for_next_block(&self) -> SequencerResult<Option<Block>>;
}

#[allow(async_fn_in_trait)]
pub trait SharedSequencer {
	async fn publish(&self, atb: AtomicTransactionBundle) -> SequencerResult<()>;

	async fn wait_for_next_block(&self) -> SequencerResult<Option<Block>>;
}
