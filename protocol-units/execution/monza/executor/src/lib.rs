pub mod v1;

use aptos_types::{
    transaction::signature_verified_transaction::SignatureVerifiedTransaction,
    block_executor::partitioner::ExecutableBlock
};
use aptos_executor_types::state_checkpoint_output::StateCheckpointOutput;
use async_channel::Sender;
use aptos_api::runtime::Apis;
use monza_execution_util::FinalityMode;


pub trait MonzaExecutor {

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
    
}
