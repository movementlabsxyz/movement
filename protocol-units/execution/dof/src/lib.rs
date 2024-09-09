mod services;
pub mod v1;

use services::Services;

pub use aptos_crypto::hash::HashValue;
pub use aptos_types::{
	block_executor::partitioner::ExecutableBlock,
	block_executor::partitioner::ExecutableTransactions,
	block_metadata::BlockMetadata,
	transaction::signature_verified_transaction::SignatureVerifiedTransaction,
	transaction::{SignedTransaction, Transaction},
};
use maptos_execution_util::config::Config;
use movement_types::block::BlockCommitment;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use std::future::Future;

#[async_trait]
pub trait DynOptFinExecutor {
	type Context: MakeOptFinServices;

	/// Initialize the background task responsible for transaction processing.
	fn background(
		&self,
		transaction_sender: Sender<SignedTransaction>,
		config: &Config,
	) -> Result<
		(Self::Context, impl Future<Output = Result<(), anyhow::Error>> + Send + 'static),
		anyhow::Error,
	>;

	/// Executes a block optimistically
	async fn execute_block_opt(
		&self,
		block: ExecutableBlock,
	) -> Result<BlockCommitment, anyhow::Error>;

	/// Update the height of the latest finalized block
	fn set_finalized_block_height(&self, block_height: u64) -> Result<(), anyhow::Error>;

	/// Revert the chain to the specified height
	async fn revert_block_head_to(&self, block_height: u64) -> Result<(), anyhow::Error>;

	/// Get block head height.
	fn get_block_head_height(&self) -> Result<u64, anyhow::Error>;

	/// Build block metadata for a timestamp
	fn build_block_metadata(
		&self,
		block_id: HashValue,
		timestamp: u64,
	) -> Result<BlockMetadata, anyhow::Error>;

	/// Rollover the genesis block
	async fn rollover_genesis_block(&self) -> Result<(), anyhow::Error>;

	/// Decrements transactions in flight on the transaction channel.
	fn decrement_transactions_in_flight(&self, count: u64);

	/// Gets the config
	fn config(&self) -> &Config;
}

pub trait MakeOptFinServices {
	fn services(&self) -> Services;
}
