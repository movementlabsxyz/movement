pub mod v1;

pub use aptos_types::{
    transaction::signature_verified_transaction::SignatureVerifiedTransaction,
    block_executor::partitioner::ExecutableBlock,
    block_executor::partitioner::ExecutableTransactions
};
pub use aptos_crypto::hash::HashValue;
use aptos_executor_types::state_checkpoint_output::StateCheckpointOutput;
use async_channel::Sender;
use aptos_api::runtime::Apis;
pub use monza_execution_util::FinalityMode;


#[tonic::async_trait]
pub trait MonzaExecutor {

    /// Runs the service
    async fn run_service(&self) -> Result<(), anyhow::Error>;

    /// Runs the necessary background tasks.
    async fn run_background_tasks(&self) -> Result<(), anyhow::Error>;

    /// Executes a block dynamically
    async fn execute_block(
        &self,
        mode : &FinalityMode, 
        block: ExecutableBlock,
    ) -> Result<StateCheckpointOutput, anyhow::Error>;

    /// Sets the transaction channel.
    async fn set_tx_channel(&self, tx_channel: Sender<SignatureVerifiedTransaction>) -> Result<(), anyhow::Error>;

    /// Gets the dyn API.
    async fn get_api(
        &self,
        mode : &FinalityMode, 
    ) -> Result<Apis, anyhow::Error>;

    /// Get block head height.
    async fn get_block_head_height(&self) -> Result<u64, anyhow::Error>;
    
}
