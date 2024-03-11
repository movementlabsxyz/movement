use tokio::sync::RwLock;
use std::sync::Arc;
use aptos_executor::block_executor::BlockExecutor;
use aptos_types::transaction::Transaction;
use aptos_vm::AptosVM;
use movement_sdk::{ExecutionLayer, Layer};
use aptos_helper_types::block::Block;


#[derive(Clone)]
pub struct AptosBlockExecutor {
    pub executor: Arc<RwLock<BlockExecutor<AptosVM>>>,
}

impl std::fmt::Debug for AptosBlockExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AptosBlockExecutor")
            .finish()
    }
}

impl AptosBlockExecutor {
    pub fn new(executor: Arc<RwLock<BlockExecutor<AptosVM>>>) -> Self {
        AptosBlockExecutor { executor }
    }
}

impl Layer for AptosBlockExecutor {}

#[async_trait::async_trait]
impl ExecutionLayer for AptosBlockExecutor {

    type Block = Block;
    type BlockId = String; // todo: change later
    type ChangeSet = u64; // todo: change later

    // Gets the next block from the previous layer.
    async fn get_next_block(
        &self
    ) -> Result<Option<Self::Block>, anyhow::Error> {
        unimplemented!();
    }

    // Executes a block and produces a change set.
    async fn execute_block(
        &self,
        block: Self::Block
    ) -> Result<Self::ChangeSet, anyhow::Error> {

        let mut executor = self.executor.write().await;
        // let parent_block_id_now = executor.committed_block_id();
   
        // todo: update for aptos 1.70
        /*// execute the block
        let output = executor
            .execute_block((block_id, block_tx.clone()), parent_block_id)
            .unwrap();

        // sign for the the ledger
        let ledger_info = LedgerInfo::new(
            BlockInfo::new(
                next_epoch,
                0,
                block_id,
                output.root_hash(),
                output.version(),
                ts,
                output.epoch_state().clone(),
            ),
            HashValue::zero(),
        );
        let li = generate_ledger_info_with_sig(&[self.signer.as_ref().unwrap().clone()], ledger_info);
        executor.commit_blocks(vec![block_id], li.clone()).unwrap();*/

        Ok(0)

    }

    // Sends a change set to the next layer,  i.e., the storage layer.
    async fn send_change_set(
        &self,
        change_set: Self::ChangeSet
    ) -> Result<(), anyhow::Error> {
        unimplemented!();
    }

    // Gets an executed block
    async fn get_block(
        &self,
        block_id: Self::BlockId
    ) -> Result<Option<Self::Block>, anyhow::Error> {
        unimplemented!();
    }

}
