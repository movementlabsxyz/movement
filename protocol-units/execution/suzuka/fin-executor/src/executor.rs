use aptos_config::config::NodeConfig;
use aptos_db::AptosDB;
use aptos_executor::{
    block_executor::BlockExecutor,
    db_bootstrapper::{generate_waypoint, maybe_bootstrap},
};
use aptos_executor_types::{state_checkpoint_output::StateCheckpointOutput, BlockExecutorTrait};
use aptos_mempool::core_mempool::CoreMempool;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{
    block_executor::config::BlockExecutorConfigFromOnchain,
    block_executor::partitioner::ExecutableBlock,
    transaction::{Transaction, WriteSetPayload},
    validator_signer::ValidatorSigner,
};
use aptos_vm::AptosVM;
use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

/// The state of `movement-network` execution can exist in three states,
/// `Dynamic`, `Optimistic`, and `Final`. The `Dynamic` state is the state.
pub enum FinalityState {
    /// The dynamic state that is subject to change and is not
    /// yet finalized. It is the state that is derived from the blocks
    /// received before any finality is reached and simply represents a
    /// local application of the fork-choice rule (longest chain)
    /// of the gossipped blocks.
    Dynamic,
    /// The optimistic state that is derived from the blocks received after DA finality.
    /// It is the state that is derived from the blocks that have been finalized by the DA.
    Optimistic,
    /// The final state that is derived from the blocks received after the finality is reached.
    Final,
}

/// The current state of the executor and its execution of blocks.
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ExecutorState {
    /// The executor is idle and waiting for a block to be executed.
    Idle,
    /// The block is executed in a speculative manner and its effects held in memory.
    Speculate,
    /// The network agrees on the block.
    Consensus,
    /// The block is committed to the state, at this point
    /// fork choices must be resolved otherwise the commitment and subsequent execution will fail.
    Commit,
}

/// The `Executor` is responsible for executing blocks and managing the state of the execution
/// against the `AptosVM`.
pub struct Executor {
    /// The executing type.
    pub block_executor: Arc<RwLock<BlockExecutor<AptosVM>>>,
    /// The current state of the executor.
    pub status: ExecutorState,
    /// The access to db.
    pub db: DbReaderWriter,
    /// The signer of the executor's transactions.
    pub signer: ValidatorSigner,
    /// The access to the core mempool.
    pub mempool: CoreMempool,
}

impl Executor {
    const DB_PATH_ENV_VAR: &'static str = "DB_DIR";

    /// Create a new `Executor` instance.
    pub fn new(
        db_dir: PathBuf,
        block_executor: BlockExecutor<AptosVM>,
        signer: ValidatorSigner,
        mempool: CoreMempool,
    ) -> Self {
        let (_aptos_db, reader_writer) = DbReaderWriter::wrap(AptosDB::new_for_test(&db_dir));
        Self {
            block_executor: Arc::new(RwLock::new(block_executor)),
            status: ExecutorState::Idle,
            db: reader_writer,
            signer,
            mempool,
        }
    }

    pub fn bootstrap_empty_db(db_dir: PathBuf) -> Result<DbReaderWriter, anyhow::Error> {
        let genesis = aptos_vm_genesis::test_genesis_change_set_and_validators(Some(1));
        let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis.0));
        let db_rw = DbReaderWriter::new(AptosDB::new_for_test(&db_dir));
        assert!(db_rw.reader.get_latest_ledger_info_option()?.is_none());

        // Bootstrap empty DB.
        let waypoint =
            generate_waypoint::<AptosVM>(&db_rw, &genesis_txn).expect("Should not fail.");
        maybe_bootstrap::<AptosVM>(&db_rw, &genesis_txn, waypoint)?;
        assert!(db_rw.reader.get_latest_ledger_info_option()?.is_some());

        Ok(db_rw)
    }

    pub fn bootstrap(
        db_dir: PathBuf,
        signer: ValidatorSigner,
        mempool: CoreMempool,
    ) -> Result<Self, anyhow::Error> {
        let db = Self::bootstrap_empty_db(db_dir)?;

        Ok(Self {
            block_executor: Arc::new(RwLock::new(BlockExecutor::new(db.clone()))),
            status: ExecutorState::Idle,
            db,
            signer,
            mempool,
        })
    }

    pub fn try_from_env() -> Result<Self, anyhow::Error> {
        // read the db dir from env or use a tempfile
        let db_dir = match std::env::var(Self::DB_PATH_ENV_VAR) {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => {
                let temp_dir = tempfile::tempdir()?;
                temp_dir.path().to_path_buf()
            }
        };

        // use the default signer, block executor, and mempool
        let signer = ValidatorSigner::random(None);
        let mempool = CoreMempool::new(&NodeConfig::default());

        Self::bootstrap(db_dir, signer, mempool)
    }

    pub fn set_commit_state(&mut self) {
        self.status = ExecutorState::Commit;
    }

    /// Execute a block which gets committed to the state.
    /// `ExecutorState` must be set to `Commit` before calling this method.
    pub async fn execute_block(
        &mut self,
        block: ExecutableBlock,
    ) -> Result<StateCheckpointOutput, anyhow::Error> {
        if self.status != ExecutorState::Commit {
            return Err(anyhow::anyhow!("Executor is not in the Commit state"));
        }

        let parent_block_id = {
            let block_executor = self.block_executor.read().map_err(|e| {
                anyhow::anyhow!("Failed to acquire block executor read lock: {:?}", e)
            })?; // acquire read lock
            block_executor.committed_block_id()
        };

        let state_checkpoint = {
            let block_executor = self.block_executor.write().map_err(|e| {
                anyhow::anyhow!("Failed to acquire block executor write lock: {:?}", e)
            })?; // acquire write lock
            block_executor.execute_and_state_checkpoint(
                block,
                parent_block_id,
                BlockExecutorConfigFromOnchain::new_no_block_limit(),
            )?
        };

        // Update the executor state
        self.status = ExecutorState::Idle;

        Ok(state_checkpoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519Signature},
        HashValue, PrivateKey, Uniform,
    };
    use aptos_types::{
        account_address::AccountAddress,
        block_executor::partitioner::ExecutableTransactions,
        chain_id::ChainId,
        transaction::{
            signature_verified_transaction::SignatureVerifiedTransaction, RawTransaction, Script,
            SignedTransaction, Transaction, TransactionPayload,
        },
    };

    fn create_signed_transaction(gas_unit_price: u64) -> SignedTransaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();

        let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
        let raw_transaction = RawTransaction::new(
            AccountAddress::random(),
            0,
            transaction_payload,
            0,
            gas_unit_price,
            0,
            ChainId::new(10), // This is the value used in aptos testing code.
        );
        SignedTransaction::new(raw_transaction, public_key, Ed25519Signature::dummy_signature())
    }

    #[tokio::test]
    async fn test_execute_block() -> Result<(), anyhow::Error> {
        let mut executor = Executor::try_from_env()?;
        executor.set_commit_state();
        let block_id = HashValue::random();
        let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
            create_signed_transaction(0),
        ));
        let txs = ExecutableTransactions::Unsharded(vec![tx]);
        let block = ExecutableBlock::new(block_id.clone(), txs);
        executor.execute_block(block).await?;
        Ok(())
    }
}
