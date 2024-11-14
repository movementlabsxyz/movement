// Implementation is split over multiple files to make the code more manageable.
// TODO: code smell, refactor the god object.
pub mod execution;
pub mod initialization;

use aptos_config::config::NodeConfig;
use aptos_crypto::HashValue;
use aptos_executor::block_executor::BlockExecutor;
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;

use tracing::info;

use maptos_execution_util::config::Config;
use movement_collections::garbage::counted::GcCounter;
use std::sync::Arc;
use tokio::sync::RwLock;

/// The `Executor` is responsible for executing blocks and managing the state of the execution
/// against the `AptosVM`.
pub struct Executor {
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

	pub async fn decrement_transactions_in_flight(&self, count: u64) {
		let mut transactions_in_flight = self.transactions_in_flight.write().await;
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
