use super::{Error, NullMempool, TransactionPipe};
use crate::executor::TxExecutionResult;
use aptos_account_whitelist::config::Config as WhitelistConfig;
use aptos_config::config::NodeConfig;
use aptos_mempool::MempoolClientRequest;
use aptos_storage_interface::DbReader;
use futures::channel::mpsc as futures_mpsc;
use maptos_execution_util::config::mempool::Config as MempoolConfig;
use movement_collections::garbage::counted::GcCounter;
use movement_da_sequencer_client::GrpcDaSequencerClient;
use movement_signer_loader::identifiers::SignerIdentifier;
use std::sync::{Arc, RwLock};
use url::Url;

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
		mempool_commit_tx_receiver: futures_mpsc::Receiver<Vec<TxExecutionResult>>, // Sender, seq number)
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
		db_reader: Arc<dyn DbReader>,
		node_config: &NodeConfig,
		mempool_config: &MempoolConfig,
		whitelist_config: &WhitelistConfig,
		transactions_in_flight: Arc<RwLock<GcCounter>>,
		transactions_in_flight_limit: Option<u64>,
		da_batch_signer: SignerIdentifier,
	) -> Result<Self, anyhow::Error> {
		Ok(Self {
			inner: BackgroundInner::Full(TransactionPipe::new(
				mempool_commit_tx_receiver,
				mempool_client_receiver,
				db_reader,
				node_config,
				mempool_config,
				whitelist_config,
				transactions_in_flight,
				transactions_in_flight_limit,
				da_batch_signer,
			)?),
		})
	}

	pub(crate) fn read_only(
		mempool_client_receiver: futures_mpsc::Receiver<MempoolClientRequest>,
	) -> Self {
		Self { inner: BackgroundInner::ReadOnly(NullMempool::new(mempool_client_receiver)) }
	}

	/// Runs the background task.
	pub async fn run(self, da_connection_url: Url) -> Result<(), Error> {
		use BackgroundInner::*;

		match self.inner {
			Full(transaction_pipe) => {
				let da_client =
					GrpcDaSequencerClient::try_connect(&da_connection_url, None, 10).await?;
				transaction_pipe.run(da_client).await
			}
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
