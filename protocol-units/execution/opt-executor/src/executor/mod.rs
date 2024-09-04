// Implementation is split over multiple files to make the code more manageable.
// TODO: code smell, refactor the god object.
pub mod execution;
pub mod initialization;

use aptos_executor::block_executor::BlockExecutor;
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;

use tracing::info;

use maptos_execution_util::config::Config;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// The `Executor` is responsible for executing blocks and managing the state of the execution
/// against the `AptosVM`.
pub struct Executor {
	/// The executing type.
	pub block_executor: Arc<BlockExecutor<AptosVM>>,
	/// The signer of the executor's transactions.
	pub signer: ValidatorSigner,
	// Shared reference on the counter of transactions in flight.
	transactions_in_flight: Arc<AtomicU64>,
	// The config for the executor.
	pub(crate) config: Config,
}

impl Executor {
	fn db(&self) -> &DbReaderWriter {
		&self.block_executor.db
	}

	pub fn db_reader(&self) -> Arc<dyn DbReader> {
		Arc::clone(&self.db().reader)
	}

	pub fn decrement_transactions_in_flight(&self, count: u64) {
		// fetch sub mind the underflow
		// a semaphore might be better here as this will rerun until the value does not change during the operation
		self.transactions_in_flight
			.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
				info!(
					target: "movement_timing",
					count,
					current,
					"decrementing_transactions_in_flight",
				);
				Some(current.saturating_sub(count))
			})
			.unwrap_or_else(|_| 0);
	}

	pub fn config(&self) -> &Config {
		&self.config
	}
}
