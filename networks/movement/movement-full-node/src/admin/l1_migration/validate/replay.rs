use crate::admin::l1_migration::validate::compare::compare_transaction_outputs;
use crate::admin::l1_migration::validate::types::api::{AptosRestClient, MovementRestClient};
use crate::admin::l1_migration::validate::types::da::{get_da_block_height, DaSequencerClient};
use anyhow::Context;
use aptos_crypto::HashValue;
use aptos_types::transaction::SignedTransaction;
use clap::{Args, Parser};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

#[derive(Parser, Debug)]
#[clap(name = "replay", about = "Stream transactions from DA-sequencer blocks")]
pub struct DaReplayTransactions {
	#[clap(value_parser)]
	#[clap(long = "movement-api", help = "The url of the Movement full node endpoint")]
	pub movement_api_url: Option<String>,
	#[clap(long = "aptos-api", help = "The url of the Aptos validator node api endpoint")]
	pub aptos_api_url: String,
	#[clap(long = "da", help = "The url of the DA-Sequencer")]
	pub da_sequencer_url: String,
	#[command(flatten)]
	da_sequencer_db: DaBlockHeight,
	#[clap(long = "diff", help = "Show diff on transaction output mismatch")]
	pub show_diff: bool,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub struct DaBlockHeight {
	#[arg(long = "da-db", help = "Path to the DA-Sequencer database")]
	pub path: Option<PathBuf>,
	#[arg(long = "da-height", help = "Synced DA-Sequencer block height")]
	pub height: Option<u64>,
}

impl DaReplayTransactions {
	pub async fn run(&self) -> anyhow::Result<()> {
		let block_height = match (self.da_sequencer_db.height, &self.da_sequencer_db.path) {
			(Some(height), _) => height,
			(_, Some(path)) => get_da_block_height(path)?,
			_ => unreachable!(),
		};
		let (tx_batches, rx_batches) = mpsc::channel::<Vec<SignedTransaction>>(10);
		let mut tasks = JoinSet::new();
		let da_sequencer_client = DaSequencerClient::try_connect(&self.da_sequencer_url).await?;
		let aptos_rest_client = AptosRestClient::try_connect(&self.aptos_api_url).await?;

		// Spawn a task which compares transaction outputs from the Movement node and Aptos node
		let tx_hashes = if let Some(ref movement_api_url) = self.movement_api_url {
			let movement_rest_client = MovementRestClient::try_connect(movement_api_url).await?;
			let (tx_hashes, rx_hashes) = mpsc::unbounded_channel::<HashValue>();
			tasks.spawn(validate_transactions(
				aptos_rest_client.clone(),
				movement_rest_client,
				rx_hashes,
				self.show_diff,
			));
			Some(tx_hashes)
		} else {
			None
		};

		// Spawn a task which submits transaction batches to the validator node
		tasks.spawn(submit_transactions(aptos_rest_client, rx_batches, tx_hashes));
		// Spawn a task which fetches transaction batches ahead
		tasks.spawn(stream_transactions(da_sequencer_client, tx_batches, block_height));

		// If one of the tasks has finished then something went wrong
		tasks.join_next().await;
		tasks.shutdown().await;

		error!("Broken stream");
		Err(anyhow::anyhow!("Broken stream"))
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	DaReplayTransactions::command().debug_assert()
}

async fn stream_transactions(
	da_sequencer_client: DaSequencerClient,
	tx_batches: mpsc::Sender<Vec<SignedTransaction>>,
	block_height: u64,
) {
	if let Ok(stream) = da_sequencer_client.stream_transactions_from_height(block_height).await {
		let stream = stream.chunks_timeout(10, Duration::from_secs(1));

		futures::pin_mut!(stream);
		while let Some(txns) = stream.next().await {
			let txns = txns
				.into_iter()
				.collect::<Result<Vec<_>, _>>()
				.context("Failed to get the next batch of Aptos transactions");

			match txns {
				Ok(txns) => {
					if tx_batches.send(txns).await.is_err() {
						// channel is closed
						break;
					}
				}
				Err(err) => {
					error!("{err}");
					break;
				}
			}
		}
		warn!("Stream of transaction from the DA-Sequencer ended unexpectedly");
	} else {
		error!("Failed to stream transactions from DA-Sequencer blocks")
	}
}

async fn submit_transactions(
	aptos_rest_client: AptosRestClient,
	mut rx_batches: mpsc::Receiver<Vec<SignedTransaction>>,
	tx_hashes: Option<mpsc::UnboundedSender<HashValue>>,
) {
	while let Some(txns) = rx_batches.recv().await {
		match aptos_rest_client.submit_batch_bcs(&txns).await {
			Ok(result) => {
				debug!("Submitted {} Aptos transaction(s)", txns.len());
				let mut failed_txns = HashSet::new();
				for failure in result.into_inner().transaction_failures {
					failed_txns.insert(failure.transaction_index);
					let txn = &txns[failure.transaction_index];
					let hash = txn.committed_hash().to_hex_literal();
					error!("Failed to submit Aptos transaction {}: {}", hash, failure.error);
				}

				if let Some(ref tx_hashes) = tx_hashes {
					if txns
						.iter()
						.enumerate()
						.filter_map(|item| match item {
							(idx, _) if failed_txns.contains(&idx) => None,
							(_, txn) => Some(txn.committed_hash()),
						})
						.try_for_each(|hash| tx_hashes.send(hash))
						.is_err()
					{
						// channel is closed
						break;
					}
				}
			}
			Err(e) => {
				error!("Failed to submit {} transaction(s): {}", txns.len(), e);
				break;
			}
		}
	}
	warn!("Stream of transaction batches ended unexpectedly");
}

async fn validate_transactions(
	aptos_rest_client: AptosRestClient,
	movement_rest_client: MovementRestClient,
	mut rx_hashes: mpsc::UnboundedReceiver<HashValue>,
	show_diff: bool,
) {
	use aptos_api_types::transaction::Transaction;

	while let Some(hash) = rx_hashes.recv().await {
		let hash_str = hash.to_hex_literal();
		let timeout = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 60;
		let result = tokio::join!(
			movement_rest_client.wait_for_transaction_by_hash(hash, timeout, None, None),
			aptos_rest_client.wait_for_transaction_by_hash(hash, timeout, None, None)
		);

		match result {
			(Ok(txn_movement), Ok(txn_aptos)) => {
				let Transaction::UserTransaction(txn_movement) = txn_movement.into_inner() else {
					unreachable!()
				};
				let Transaction::UserTransaction(txn_aptos) = txn_aptos.into_inner() else {
					unreachable!()
				};

				match compare_transaction_outputs(*txn_movement, *txn_aptos, show_diff) {
					Ok(valid) if valid => info!("Validated transaction {}", hash_str),
					Ok(_) => {} // invalid, errors logged elsewhere
					Err(e) => error!("Failed to validate transaction {}: {}", hash_str, e),
				}
			}
			(Ok(_), Err(error_aptos)) => {
				error!(
					"The execution of the transaction {} failed on Aptos: {}",
					hash_str, error_aptos
				)
			}
			(Err(error_movement), Ok(_)) => error!(
				"The execution of the transaction {} failed on Movement but succeeded on Aptos: {}",
				hash_str, error_movement
			),
			_ => {
				// ignore if the execution failed on both sides???
			}
		}
	}
	warn!("Stream of transaction hashes ended unexpectedly");
}
