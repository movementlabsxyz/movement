use movement_types::{
	atomic_transaction_bundle::AtomicTransactionBundle, block::Block, transaction::Transaction,
};

pub trait Sequencer {
	async fn publish_many(&self, atbs: Vec<Transaction>) -> Result<(), anyhow::Error>;

	async fn publish(&self, atb: Transaction) -> Result<(), anyhow::Error>;

	async fn wait_for_next_block(&self) -> Result<Option<Block>, anyhow::Error>;

	/// Removes outdated transactions that did not make it into a block.
	///
	/// This asynchronous task should be called periodically in a loop.
	/// It takes care of observing a sleeping period to separate garbage collection sweeps.
	async fn gc(&self) -> Result<(), anyhow::Error>;
}

pub trait SharedSequencer {
	async fn publish(&self, atb: AtomicTransactionBundle) -> Result<(), anyhow::Error>;

	async fn wait_for_next_block(&self) -> Result<Option<Block>, anyhow::Error>;
}
