use crate::*;
use monza_opt_executor::Executor;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MonzaExecutorV1 {
    pub executor: Arc<RwLock<Executor>>,
}

impl MonzaExecutorV1 {
    pub fn new(executor : Executor) -> Self {
        Self {
            executor: Arc::new(RwLock::new(executor)),
        }
    }
}

#[tonic::async_trait]
impl MonzaExecutor for MonzaExecutorV1 {
    
    /// Executes a block dynamically
    async fn execute_block(
        &self,
        mode : &FinalityMode, 
        block: ExecutableBlock,
    ) -> Result<StateCheckpointOutput, anyhow::Error> {

        match mode {
            FinalityMode::Dyn => unimplemented!(),
            FinalityMode::Opt => {
                let mut executor = self.executor.write().await;
                executor.set_commit_state();
                executor.execute_block(block).await
            },
            FinalityMode::Fin => unimplemented!(),
        }

    }

    /// Sets the transaction channel.
    async fn set_tx_channel(&self, _tx_channel: Sender<SignatureVerifiedTransaction>) -> Result<(), anyhow::Error> {
        unimplemented!()
    }

    /// Gets the dyn API.
    async fn get_api(
        &self,
        _mode : &FinalityMode, 
    ) -> Result<Apis, anyhow::Error> {
        unimplemented!()
    }

    /// Get block head height.
    async fn get_block_head_height(&self) -> Result<u64, anyhow::Error> {
        unimplemented!()
    }

}