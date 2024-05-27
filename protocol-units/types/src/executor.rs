use async_channel::Sender;

use crate::Apis;
use crate::ExecutableBlock;
use crate::FinalityMode;
use crate::SignedTransaction;

#[async_trait::async_trait]
pub trait ExecutorOps {
	/// Runs the service
	async fn run_service(&self) -> Result<(), anyhow::Error>;

	/// Runs the necessary background tasks.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error>;

	/// Executes a block dynamically
	async fn execute_block(
		&self,
		mode: FinalityMode,
		block: ExecutableBlock,
	) -> Result<(), anyhow::Error>;

	/// Sets the transaction channel.
	fn set_tx_channel(&mut self, tx_channel: Sender<SignedTransaction>);

	/// Gets the dyn API.
	fn get_api(&self, mode: FinalityMode) -> Apis;

	/// Get block head height.
	async fn get_block_head_height(&self) -> Result<u64, anyhow::Error>;
}
