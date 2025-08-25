use crate::types::api::AptosRestClient;
use crate::types::da::{DaSequencerClient, DaSequencerDb};
use anyhow::Context;
use aptos_types::transaction::SignedTransaction;
use clap::{Args, Parser};
use movement_types::block::Block;
use std::path::PathBuf;
use tokio_stream::StreamExt;
use tracing::{error, info};

#[derive(Parser)]
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

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct DaBlockHeight {
	#[arg(long = "da-db", help = "Path to the DA-Sequencer database")]
	pub path: Option<PathBuf>,
	#[arg(long = "da-height", help = "Synced DA-Sequencer block height")]
	pub height: Option<u64>,
}

impl DaReplayTransactions {
	pub async fn run(self) -> anyhow::Result<()> {
		let block_height = match (self.da_sequencer_db.height, self.da_sequencer_db.path) {
			(Some(height), _) => height,
			(_, Some(ref path)) => get_da_block_height(path)?,
			_ => unreachable!(),
		};
		let da_sequencer_client = DaSequencerClient::try_connect(&self.da_sequencer_url).await?;
		let rest_client = AptosRestClient::new(&self.aptos_api_url)?;
		stream_transactions(&rest_client, &da_sequencer_client, block_height + 1).await
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
	rest_client: &AptosRestClient,
	da_sequencer_client: &DaSequencerClient,
	block_height: u64,
) -> anyhow::Result<()> {
	let mut blocks = da_sequencer_client
		.stream_blocks_from_height(0)
		.await
		.context("Failed to stream blocks from DA")?;

	info!("streaming blocks from DA, starting at block_height: {}", block_height);

	if let Some(block_res) = blocks.next().await {
		let block = block_res.context("Failed to get next block from DA")?;
		info!("block at DA height {}: 0x{}", block.height, hex::encode(block.block_id));
		let block = bcs::from_bytes::<'_, Block>(block.data.as_ref())
			.context("Failed to deserialize Movement block")?;

		for transaction in block.transactions() {
			info!("processing transaction 0x{}", transaction.id());
			let aptos_transaction = bcs::from_bytes::<'_, SignedTransaction>(transaction.data())
				.context("Failed to deserialize Aptos transaction")?;

			info!(
				"Submitting Aptos transaction {}",
				aptos_transaction.committed_hash().to_hex_literal()
			);
			rest_client
				.submit_bcs(&aptos_transaction)
				.await
				.context("Failed to submit the Aptos transaction")?;
		}
	}

	error!("Broken DA stream");
	Err(anyhow::anyhow!("Broken DA stream"))
}
