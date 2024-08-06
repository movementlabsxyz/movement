// Implementation is split over multiple files to make the code more manageable.
// TODO: code smell, refactor the god object.
pub mod execution;
pub mod initialization;

use aptos_config::config::NodeConfig;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;

use anyhow::Context as _;
use tracing::info;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// The `Executor` is responsible for executing blocks and managing the state of the execution
/// against the `AptosVM`.
pub struct Executor {
	/// The executing type.
	pub block_executor: Arc<BlockExecutor<AptosVM>>,
	/// The access to db.
	pub db: DbReaderWriter,
	/// The signer of the executor's transactions.
	pub signer: ValidatorSigner,
	/// The configuration of the node.
	pub node_config: NodeConfig,
	/// Maptos config
	pub maptos_config: maptos_execution_util::config::Config,
	// Shared reference on the counter of transactions in flight.
	transactions_in_flight: Arc<AtomicU64>,
}

impl Executor {
	/// Create a new `Executor` instance and a future to run the background
	/// tasks.
	pub fn try_new(
		block_executor: BlockExecutor<AptosVM>,
		signer: ValidatorSigner,
		node_config: NodeConfig,
		maptos_config: maptos_execution_util::config::Config,
	) -> Result<Self, anyhow::Error> {
		let (_aptos_db, reader_writer) = DbReaderWriter::wrap(AptosDB::new_for_test(
			&maptos_config.chain.maptos_db_path.clone().context("No db path provided.")?,
		));
		Ok(Self {
			block_executor: Arc::new(block_executor),
			db: reader_writer,
			signer,
			node_config,
			maptos_config,
			transactions_in_flight: Arc::new(AtomicU64::new(0)),
		})
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
}
