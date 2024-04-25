use crate::*;
use monza_opt_executor::Executor;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_channel::Sender;
use aptos_types::transaction::SignedTransaction;

#[derive(Clone)]
pub struct MonzaExecutorV1 {
    // this rwlock may be somewhat redundant
    pub executor: Arc<RwLock<Executor>>,
    pub transaction_channel: Sender<SignedTransaction>,
}

impl MonzaExecutorV1 {
    pub fn new(executor : Executor, transaction_channel: Sender<SignedTransaction>) -> Self {
        Self {
            executor: Arc::new(RwLock::new(executor)),
            transaction_channel,
        }
    }
}

#[tonic::async_trait]
impl MonzaExecutor for MonzaExecutorV1 {

    /// Runs the service.
    async fn run_service(&self) -> Result<(), anyhow::Error> {
        let executor = self.executor.read().await;
        executor.run_service().await
    }

    /// Runs the necessary background tasks.
    async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
        
        loop {
            // readers should be able to run concurrently
            let executor = self.executor.read().await;
            executor.tick_mempool_pipe(self.transaction_channel.clone()).await?;
        }

        Ok(())

    }
    
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

    /// Gets the API.
    async fn get_api(
        &self,
        _mode : &FinalityMode, 
    ) -> Result<Apis, anyhow::Error> {
        match _mode {
            FinalityMode::Dyn => unimplemented!(),
            FinalityMode::Opt => {
                let executor = self.executor.read().await;
                Ok(executor.try_get_apis().await?)
            },
            FinalityMode::Fin => unimplemented!(),
        }
    }

    /// Get block head height.
    async fn get_block_head_height(&self) -> Result<u64, anyhow::Error> {
        unimplemented!()
    }

}