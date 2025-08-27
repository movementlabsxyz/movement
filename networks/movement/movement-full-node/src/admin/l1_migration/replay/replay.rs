use crate::admin::l1_migration::replay::types::api::AptosRestClient;
use crate::admin::l1_migration::replay::types::da::{DaSequencerClient, DaSequencerDb};
use anyhow::Context;
use aptos_types::transaction::SignedTransaction;
use clap::{Args, Parser};
use futures::pin_mut;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::StreamExt;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[clap(name = "replay", about = "Stream transactions from DA-sequencer blocks")]
pub struct DaReplayTransactions {
	#[clap(value_parser)]
	#[clap(long = "api", help = "The url of the Aptos validator node api endpoint")]
	pub aptos_api_url: String,
	#[clap(long = "da", help = "The url of the DA-Sequencer")]
	pub da_sequencer_url: String,
	#[command(flatten)]
	da_sequencer_db: DaBlockHeight,
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
		let da_sequencer_client = DaSequencerClient::try_connect(&self.da_sequencer_url).await?;
		let rest_client = AptosRestClient::new(&self.aptos_api_url)?;
		stream_transactions(rest_client, da_sequencer_client, block_height + 1).await
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	DaReplayTransactions::command().debug_assert()
}

fn get_da_block_height(path_buf: &PathBuf) -> Result<u64, anyhow::Error> {
	let db = DaSequencerDb::open(path_buf)?;
	db.get_synced_height()
}

async fn stream_transactions(
	rest_client: AptosRestClient,
	da_sequencer_client: DaSequencerClient,
	block_height: u64,
) -> anyhow::Result<()> {
	let stream = da_sequencer_client
		.stream_transactions_from_height(block_height)
		.await?
		.chunks_timeout(10, Duration::from_secs(1));
	let (tx, mut rx) = mpsc::channel::<Vec<SignedTransaction>>(10);
	let mut tasks = JoinSet::new();

	// Spawn a task which fetches transaction batches ahead
	tasks.spawn(async move {
		pin_mut!(stream);
		while let Some(txns) = stream.next().await {
			let txns = txns
				.into_iter()
				.collect::<Result<Vec<_>, _>>()
				.context("Failed to get the next batch of Aptos transactions");

			match txns {
				Ok(txns) => {
					if let Err(_) = tx.send(txns).await {
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
	});

	// Spawn a task which submits transaction batches to the validator node
	tasks.spawn(async move {
		while let Some(txns) = rx.recv().await {
			match rest_client.submit_batch_bcs(&txns).await {
				Ok(result) => {
					info!("Submitted {} Aptos transaction(s)", txns.len());
					for failure in result.into_inner().transaction_failures {
						let txn = &txns[failure.transaction_index];
						let hash = txn.committed_hash().to_hex_literal();
						error!("Failed to submit Aptos transaction {}: {}", hash, failure.error);
					}
				}
				Err(e) => {
					error!("Failed to submit {} transaction(s): {}", txns.len(), e);
					break;
				}
			}
		}
	});

	// If one of the tasks has finished then something went wrong
	tasks.join_next().await;
	tasks.shutdown().await;

	error!("Broken DA stream");
	Err(anyhow::anyhow!("Broken DA stream"))
}
