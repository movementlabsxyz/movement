// Implementation is split over multiple files to make the code more manageable.
// TODO: code smell, refactor the god object.
pub mod execution;
pub mod initialization;

use aptos_config::config::NodeConfig;
use aptos_crypto::HashValue;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor_types::StateComputeResult;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::ledger_info::LedgerInfoWithSignatures;
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_types::transaction::TransactionStatus;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use maptos_execution_util::config::Config;
use movement_collections::garbage::counted::GcCounter;
use std::cmp::Ordering;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;

// Store the ledger state after the block at the given height has been executed.
// This differs from `BlockCommitment`, which only indicates that a block with a specific ID was executed.
// `ExecutionState` is used to compare ledger states at the same height.
#[derive(Debug, Clone)]
pub struct ExecutionState {
	pub block_height: u64,
	pub ledger_timestamp: u64,
	pub ledger_version: u64,
}

impl ExecutionState {
	pub fn build(ledger_info: &LedgerInfoWithSignatures, block_height: u64) -> Self {
		let ledger_info = ledger_info.ledger_info();
		ExecutionState {
			block_height,
			ledger_timestamp: ledger_info.timestamp_usecs().into(),
			ledger_version: ledger_info.version().into(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct TxExecutionResult {
	pub hash: HashValue,
	pub sender: AccountAddress,
	pub seq_number: u64,
	pub status: TransactionStatus,
}

impl TxExecutionResult {
	pub fn new(
		hash: HashValue,
		sender: AccountAddress,
		seq_number: u64,
		status: TransactionStatus,
	) -> Self {
		TxExecutionResult { hash, sender, seq_number, status }
	}

	pub fn merge_result(
		user_txns: Vec<(HashValue, AccountAddress, u64)>,
		result: &StateComputeResult,
	) -> Vec<TxExecutionResult> {
		let compute_status = result.compute_status_for_input_txns();

		// This code has been copied form Aptos core, file state_compute.rs, function schedule_compute()
		// the length of compute_status is user_txns.len() + num_vtxns + 1 due to having blockmetadata
		// Change => into a > because the user_txns doesn't contains the first block meta data Tx.
		if user_txns.len() > compute_status.len() {
			// reconfiguration suffix blocks don't have any transactions
			// otherwise, this is an error
			if !compute_status.is_empty() {
				tracing::error!(
                        "Expected compute_status length and actual compute_status length mismatch! user_txns len: {}, compute_status len: {}, has_reconfiguration: {}",
                        user_txns.len(),
                        compute_status.len(),
                        result.has_reconfiguration(),
                    );
			}
			vec![]
		} else {
			let user_txn_status = &compute_status[compute_status.len() - user_txns.len()..];
			user_txns
				.into_iter()
				.zip(user_txn_status)
				//remove all non user tx.
				.filter(|((tx_hash, _, _), _)| tx_hash != &HashValue::zero())
				.map(|((tx_hash, sender, seq_num), status)| {
					TxExecutionResult::new(tx_hash, sender, seq_num, status.clone())
				})
				.collect()
		}
	}
}

impl PartialEq for TxExecutionResult {
	fn eq(&self, other: &Self) -> bool {
		self.hash == other.hash
	}
}

impl Eq for TxExecutionResult {}

impl Ord for TxExecutionResult {
	fn cmp(&self, other: &Self) -> Ordering {
		self.hash.cmp(&other.hash)
	}
}

impl PartialOrd for TxExecutionResult {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.hash.cmp(&other.hash))
	}
}

impl Hash for TxExecutionResult {
	fn hash<H: Hasher>(&self, state: &mut H) {
		// Tx exec result are uniq per their hash.
		// Tx are executed only one time.
		self.hash.hash(state);
	}
}

// Executor channel size.
// Allow 2^16 transactions before appling backpressure given theoretical maximum TPS of 170k.
pub const EXECUTOR_CHANNEL_SIZE: usize = 2_usize.pow(16);

/// The `Executor` is responsible for executing blocks and managing the state of the execution
/// against the `AptosVM`.
pub struct Executor {
	/// Send commited Tx
	pub mempool_tx_exec_result_sender: UnboundedSender<Vec<TxExecutionResult>>,
	/// The executing type.
	pub block_executor: Arc<BlockExecutor<AptosVM>>,
	/// The signer of the executor's transactions.
	pub signer: ValidatorSigner,
	// Shared reference on the counter of transactions in flight.
	transactions_in_flight: Arc<RwLock<GcCounter>>,
	// The config for the executor.
	pub(crate) config: Config,
	/// The node config derived from the maptos config.
	pub(crate) node_config: NodeConfig,
}

impl Executor {
	fn db(&self) -> &DbReaderWriter {
		&self.block_executor.db
	}

	pub fn db_reader(&self) -> Arc<dyn DbReader> {
		Arc::clone(&self.db().reader)
	}

	pub fn decrement_transactions_in_flight(&self, count: u64) {
		// unwrap because lock is poisoned
		let mut transactions_in_flight = self.transactions_in_flight.write().unwrap();
		let current = transactions_in_flight.get_count();
		info!(
			target: "movement_timing",
			count,
			current,
			"decrementing_transactions_in_flight",
		);
		transactions_in_flight.decrement(count);
	}

	pub fn config(&self) -> &Config {
		&self.config
	}

	pub fn has_executed_transaction(
		&self,
		transaction_hash: HashValue,
	) -> Result<bool, anyhow::Error> {
		let reader = self.db_reader();
		let ledger_version = reader.get_latest_ledger_info_version()?;
		match reader.get_transaction_by_hash(transaction_hash, ledger_version, false)? {
			Some(_) => Ok(true),
			None => Ok(false),
		}
	}
}
