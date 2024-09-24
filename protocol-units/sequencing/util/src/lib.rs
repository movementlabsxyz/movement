use movement_types::{
	atomic_transaction_bundle::AtomicTransactionBundle, block::Block, transaction::Transaction,
};

pub trait Sequencer {
	async fn publish_many(&mut self, atbs: Vec<Transaction>) -> Result<(), anyhow::Error>;

	async fn publish(&mut self, atb: Transaction) -> Result<(), anyhow::Error>;

	async fn wait_for_next_block(&mut self) -> Result<Option<Block>, anyhow::Error>;
}

pub trait SharedSequencer {
	async fn publish(&self, atb: AtomicTransactionBundle) -> Result<(), anyhow::Error>;

	async fn wait_for_next_block(&self) -> Result<Option<Block>, anyhow::Error>;
}
