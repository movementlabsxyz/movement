use super::{Error, NullMempool, TransactionPipe};

use maptos_execution_util::config::mempool::Config as MempoolConfig;

use aptos_config::config::NodeConfig;
use aptos_mempool::MempoolClientRequest;
use aptos_storage_interface::DbReader;
use aptos_types::transaction::SignedTransaction;

use futures::channel::mpsc as futures_mpsc;
use movement_collections::garbage::counted::GcCounter;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// The background task for the executor, processing the incoming transactions
/// in a mempool. If the executor is configured in the read-only mode,
/// a stub task needs to be run to provide integration with aptos API services.
pub struct BackgroundTask {
	inner: BackgroundInner,
}

enum BackgroundInner {
	Full(TransactionPipe),
	ReadOnly(NullMempool),
}

impl BackgroundTask {
	/// Constructs the full background tasks for transaction processing.
	pub(crate) fn transaction_pipe(
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		transaction_sender: mpsc::Sender<SignedTransaction>,
		db_reader: Arc<dyn DbReader>,
		node_config: &NodeConfig,
		mempool_config: &MempoolConfig,
		transactions_in_flight: Arc<RwLock<GcCounter>>,
		transactions_in_flight_limit: Option<u64>,
	) -> Self {
		Self {
			inner: BackgroundInner::Full(TransactionPipe::new(
				mempool_client_receiver,
				transaction_sender,
				db_reader,
				node_config,
				mempool_config,
				transactions_in_flight,
				transactions_in_flight_limit,
			)),
		}
	}

	pub(crate) fn read_only(
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
	) -> Self {
		Self { inner: BackgroundInner::ReadOnly(NullMempool::new(mempool_client_receiver)) }
	}

	/// Runs the background task.
	pub async fn run(self) -> Result<(), Error> {
		use BackgroundInner::*;

		match self.inner {
			Full(transaction_pipe) => transaction_pipe.run().await,
			ReadOnly(null_mempool) => null_mempool.run().await,
		}
	}

	/// A test helper to extract the transaction pipe task.
	///
	/// # Panics
	///
	/// If the background task has been created as read-only,
	/// this function panics
	pub fn into_transaction_pipe(self) -> TransactionPipe {
		use BackgroundInner::*;

		match self.inner {
			Full(task) => task,
			ReadOnly(_) => panic!("task has been created as read-only"),
		}
	}
}
