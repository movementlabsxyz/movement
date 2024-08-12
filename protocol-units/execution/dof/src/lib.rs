pub mod v1;

use aptos_api::runtime::Apis;
pub use aptos_crypto::hash::HashValue;
pub use aptos_types::{
	block_executor::partitioner::ExecutableBlock,
	block_executor::partitioner::ExecutableTransactions,
	block_metadata::BlockMetadata,
	transaction::signature_verified_transaction::SignatureVerifiedTransaction,
	transaction::{SignedTransaction, Transaction},
};

use movement_types::BlockCommitment;

use async_channel::Sender;
use async_trait::async_trait;

#[async_trait]
pub trait DynOptFinExecutor {
	/// Runs the service
	async fn run_service(&self) -> Result<(), anyhow::Error>;

	/// Runs the necessary background tasks.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error>;

	/// Executes a block optimistically
	async fn execute_block_opt(
		&self,
		block: ExecutableBlock,
	) -> Result<BlockCommitment, anyhow::Error>;

	/// Update the height of the latest finalized block
	fn set_finalized_block_height(&self, block_height: u64) -> Result<(), anyhow::Error>;

	/// Sets the transaction channel.
	fn set_tx_channel(&mut self, tx_channel: Sender<SignedTransaction>);

	/// Gets the API for the opt (optimistic) state.
	fn get_opt_apis(&self) -> Apis;

	/// Gets the API for the fin (finalized) state.
	fn get_fin_apis(&self) -> Apis;

	/// Get block head height.
	async fn get_block_head_height(&self) -> Result<u64, anyhow::Error>;

	/// Build block metadata for a timestamp
	async fn build_block_metadata(
		&self,
		block_id: HashValue,
		timestamp: u64,
	) -> Result<BlockMetadata, anyhow::Error>;

	/// Rollover the genesis block
	async fn rollover_genesis_block(&self) -> Result<(), anyhow::Error>;

	/// Decrements transactions in flight on the transaction channel.
	fn decrement_transactions_in_flight(&self, count: u64);
}
