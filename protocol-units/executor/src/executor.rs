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
use std::sync::RwLock;

/// The name that appends the dir path of the rocksdb.
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
	pub block_executor: BlockExecutor<AptosVM>,
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
	/// Create a new `Executor` instance.
	pub fn new(
		block_executor: BlockExecutor<AptosVM>,
		signer: ValidatorSigner,
		mempool: CoreMempool,
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
		let parent_block_id = self.block_executor.committed_block_id();
		log::info!("Executing block: {:?}", block.block_id);
		let state_checkpoint = self.block_executor.execute_and_state_checkpoint(
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
	use aptos_config::config::NodeConfig;
	use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
	use aptos_crypto::{HashValue, PrivateKey, Uniform};
	use aptos_executor::block_executor::BlockExecutor;
	use aptos_storage_interface::DbReaderWriter;
	use aptos_types::account_address::AccountAddress;
	use aptos_types::block_executor::partitioner::ExecutableTransactions;
	use aptos_types::chain_id::ChainId;
	use aptos_types::transaction::signature_verified_transaction::SignatureVerifiedTransaction;
	use aptos_types::transaction::{
		RawTransaction, Script, SignedTransaction, Transaction, TransactionPayload,
	};
	use aptos_types::validator_signer::ValidatorSigner;

	fn init_executor() -> Executor {
		let (_, reader_writer) = DbReaderWriter::wrap(AptosDB::new_for_test(""));
		let block_executor = BlockExecutor::new(reader_writer);
		let signer = ValidatorSigner::random(None);
		let mempool = CoreMempool::new(&NodeConfig::default());
		Executor::new(block_executor, signer, mempool)
	}

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

	#[test]
	fn test_executor_new() {
		let executor = init_executor();
		assert_eq!(executor.status, ExecutorState::Idle);
	}

	#[tokio::test]
	async fn test_execute_block() {
		let mut executor = init_executor();
		let block_id = HashValue::random();
		let tx = SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(
			create_signed_transaction(0),
		));
		let txs = ExecutableTransactions::Unsharded(vec![tx]);
		let block = ExecutableBlock::new(block_id.clone(), txs);
		executor.execute_block(block).await.unwrap();
	}
}
