use std::fmt::Debug;

use sui_helper_types::{
    providers::{
        gas_info::GasInfoProvider,
        input_object::InputObjectProvider,
        epoch::EpochProvider,
        verified_executable_transaction::VerifiedExecutableBlockProvider,
        object_version::ObjectVersionProvider
    },
    block::{Block, VerifiedExecutableBlock}
};

use movement_sdk::{Layer, ExecutionLayer};
use sui_types::storage::BackingStore;
use std::sync::Arc;
use sui_types::executable_transaction::VerifiedExecutableTransaction;

/// Sui block executor struct.
/// ? Feel free to change the ref types to whatever you want.

#[derive(Clone)]
pub struct SuiBlockExecutor {
    backing_store : Arc<dyn BackingStore + Send + Sync>,
    epoch_provider : Arc<dyn EpochProvider + Send + Sync>,
    gas_info_provider : Arc<dyn GasInfoProvider + Send + Sync>,
    input_object_provider : Arc<dyn InputObjectProvider + Send + Sync>,
    verified_executable_block_provider : Arc<dyn VerifiedExecutableBlockProvider + Send + Sync>,
    object_version_provider : Arc<dyn ObjectVersionProvider + Send + Sync>
}


impl Debug for SuiBlockExecutor {
    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuiBlockExecutor")
            .finish()
    }

}

impl SuiBlockExecutor {

    /// Creates a new sui block executor.
    pub fn new(
        backing_store : Arc<dyn BackingStore + Send + Sync>,
        epoch_provider : Arc<dyn EpochProvider + Send + Sync>,
        gas_info_provider : Arc<dyn GasInfoProvider + Send + Sync>,
        input_object_provider : Arc<dyn InputObjectProvider + Send + Sync>,
        verified_executable_block_provider : Arc<dyn VerifiedExecutableBlockProvider + Send + Sync>,
        object_version_provider : Arc<dyn ObjectVersionProvider + Send + Sync>
    ) -> Self {
        Self {
            backing_store,
            epoch_provider,
            gas_info_provider,
            input_object_provider,
            verified_executable_block_provider,
            object_version_provider
        }
    }

    async fn execute_transaction_group(
        &self,
        transaction_group : Vec<VerifiedExecutableTransaction>
    ) -> Result<(), anyhow::Error> {
        for transaction in transaction_group {
            // todo: use execute_transaction_to_effects
        }
        unimplemented!();
    }

}

impl Layer for SuiBlockExecutor {

}

#[async_trait::async_trait]
impl ExecutionLayer for SuiBlockExecutor {

    type Block = Block;
    type BlockId = String; // todo: will update this
    type ChangeSet = Option<u64>; // todo: will update this

    // Gets the next block from the previous layer.
    async fn get_next_block(
        &self
    ) -> Result<Option<Self::Block>, anyhow::Error> {
        unimplemented!(); // ? Don't worry about this for now.
    }

    // Executes a block and produces a change set.
    async fn execute_block(
        &self,
        block: Self::Block
    ) -> Result<Self::ChangeSet, anyhow::Error> {

        // transform the block to a verified executable block
        let verified_executable_block = self.verified_executable_block_provider.verified_executable_block(&block).await?;

        // get the max parallel groups
        let max_parallel_groups = verified_executable_block.get_max_parallel_groups();

        // set up the object versions for the transactions
        let sequencer_parallel_groups = self.object_version_provider.assign_shared_object_versions(max_parallel_groups).await?;

        // execute the transaction groups in parallel
        futures::future::try_join_all(
            sequencer_parallel_groups.into_iter().map(|transaction_group| self.execute_transaction_group(transaction_group))
        ).await?;

        unimplemented!(); // ! Worry about this for now.

    }

    // Sends a change set to the next layer,  i.e., the storage layer.
    async fn send_change_set(
        &self,
        change_set: Self::ChangeSet
    ) -> Result<(), anyhow::Error> {
        unimplemented!(); // ? Don't worry about this for now.
    }

    // Gets an executed block
    async fn get_block(
        &self,
        block_id: Self::BlockId
    ) -> Result<Option<Self::Block>, anyhow::Error> {
        unimplemented!(); // ? Don't worry about this for now.
    }

}

