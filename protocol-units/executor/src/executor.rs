use crate::types::Block;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor_types::state_checkpoint_output::StateCheckpointOutput;
use aptos_executor_types::BlockExecutorTrait;
use aptos_mempool::core_mempool::CoreMempool;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::block_executor::config::BlockExecutorConfigFromOnchain;
use aptos_types::block_executor::partitioner::ExecutableBlock;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use std::sync::{Arc, RwLock};

const APTOS_DB_DIR: &str = ".aptosdb-block-executor";

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
	pub signer: Option<ValidatorSigner>,
	/// The access to the core mempool.
	pub mempool: Arc<RwLock<CoreMempool>>,
}

impl Executor {
	/// Create a new `Executor` instance.
	pub fn new(
		block_executor: Arc<RwLock<BlockExecutor<AptosVM>>>,
		signer: Option<ValidatorSigner>,
		mempool: Arc<RwLock<CoreMempool>>,
	) -> Self {
		let path = format!("{}/{}", dirs::home_dir().unwrap().to_str().unwrap(), APTOS_DB_DIR);
		let (_aptos_db, reader_writer) = DbReaderWriter::wrap(AptosDB::new_for_test(path.as_str()));
		Self { block_executor, status: ExecutorState::Idle, db: reader_writer, signer, mempool }
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
		let executor = self.block_executor.write().unwrap();
		let parent_block_id = executor.committed_block_id();
		log::info!("Executing block: {:?}", block.block_id);
		let state_checkpoint = executor.execute_and_state_checkpoint(
			block,
			parent_block_id,
			BlockExecutorConfigFromOnchain::new_no_block_limit(),
		)?;

		// Update the executor state
		self.status = ExecutorState::Idle;

		Ok(state_checkpoint)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use aptos_crypto::ed25519::Ed25519PrivateKey;
	use aptos_types::{
		block_info::BlockInfo,
		ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
		validator_signer::ValidatorSigner,
	};
	use aptos_vm::AptosVM;
	use executor::block_executor::BlockExecutor;
	use std::sync::Arc;

	#[test]
	fn test_executor_new() {
		let block_executor = Arc::new(RwLock::new(BlockExecutor::<AptosVM>::new(Arc::new(
			DbReaderWriter::new(DbReader::new(Arc::new(MockTreeStore::default())), None),
		))));
		let signer =
			Some(ValidatorSigner::new(Vec::new(), Ed25519PrivateKey::generate_for_testing()));
		let mempool = Arc::new(RwLock::new(CoreMempool::new(Arc::new(MockDB::default()))));

		let executor = Executor::new(block_executor, signer, mempool);

		assert_eq!(executor.status, ExecutorState::Idle);
		assert!(executor.signer.is_some());
	}

	#[tokio::test]
	async fn test_execute_block() {
		let block_executor = Arc::new(RwLock::new(BlockExecutor::<AptosVM>::new(Arc::new(
			DbReaderWriter::new(DbReader::new(Arc::new(MockTreeStore::default())), None),
		))));
		let signer =
			Some(ValidatorSigner::new(Vec::new(), Ed25519PrivateKey::generate_for_testing()));
		let mempool = Arc::new(RwLock::new(CoreMempool::new(Arc::new(MockDB::default()))));

		let mut executor = Executor::new(block_executor, signer, mempool);

		// Create a sample executable block
		let block = ExecutableBlock { block: BlockInfo::random(), txns: vec![] };

		// Try executing the block when executor is in Idle state
		let result = executor.execute_block(block.clone()).await;
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().to_string(), "Executor is not in the Commit state");

		// Set the executor state to Commit
		executor.status = ExecutorState::Commit;

		// Execute the block
		let result = executor.execute_block(block).await;
		assert!(result.is_ok());

		// Check if the executor state is updated to Idle
		assert_eq!(executor.status, ExecutorState::Idle);
	}
}
