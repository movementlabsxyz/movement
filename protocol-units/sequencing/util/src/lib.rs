use mempool_util::{MempoolBlockOperationsError, MempoolTransactionOperationsError};
use movement_types::{AtomicTransactionBundle, Block, Transaction};
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SequencerError<E> {
	#[error("MempoolTransactionOperationsError error: {0}")]
	MempoolTransactionOperationsError(#[from] MempoolTransactionOperationsError),
	#[error("MempoolBlockOperationsError error: {0}")]
	MempoolBlockOperationsError(#[from] MempoolBlockOperationsError<E>),
}

pub type SequencerResult<T, E> = Result<T, SequencerError<E>>;

#[allow(async_fn_in_trait)]
pub trait Sequencer<E> {
	async fn publish(&self, atb: Transaction) -> SequencerResult<(), E>;

	async fn wait_for_next_block(&self) -> SequencerResult<Option<Block>, E>;
}

#[allow(async_fn_in_trait)]
pub trait SharedSequencer<E> {
	async fn publish(&self, atb: AtomicTransactionBundle) -> SequencerResult<(), E>;

	async fn wait_for_next_block(&self) -> SequencerResult<Option<Block>, E>;
}
