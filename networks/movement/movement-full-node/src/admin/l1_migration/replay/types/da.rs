use crate::node::da_db::DaDB;
use anyhow::Context;
use aptos_types::transaction::SignedTransaction;
use futures::Stream;
use futures::TryStreamExt;
use movement_da_sequencer_client::DaSequencerClient as _;
use movement_da_sequencer_client::{GrpcDaSequencerClient, StreamReadBlockFromHeight};
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use std::path::Path;
use tokio::sync::RwLock;
use tracing::debug;

pub struct DaSequencerClient(RwLock<GrpcDaSequencerClient>);

impl DaSequencerClient {
	pub async fn try_connect(url: &str) -> Result<Self, anyhow::Error> {
		let client = GrpcDaSequencerClient::try_connect(
			&url.parse()
				.map_err(|e| anyhow::anyhow!("Failed to parse DA-Sequencer url: {}", e))?,
			10,
		)
		.await?;
		Ok(Self(RwLock::new(client)))
	}

	pub async fn stream_blocks_from_height(
		&self,
		block_height: u64,
	) -> Result<StreamReadBlockFromHeight, anyhow::Error> {
		let mut client = self.0.write().await;
		let request = StreamReadFromHeightRequest { height: block_height };
		let result = client.stream_read_from_height(request).await;
		let (blocks, _) = result?;
		Ok(blocks)
	}

	pub async fn stream_transactions_from_height(
		&self,
		block_height: u64,
	) -> Result<impl Stream<Item = Result<SignedTransaction, anyhow::Error>> + Send, anyhow::Error>
	{
		let mut blocks = self
			.stream_blocks_from_height(block_height)
			.await
			.context("Failed to stream blocks from DA")?;

		let stream = async_stream::try_stream! {
			while let Some(da_block) = blocks.try_next().await.context("Failed to get next block from DA")? {
				let block = bcs::from_bytes::<'_, movement_types::block::Block>(da_block.data.as_ref())
					.context("Failed to deserialize Movement block")?;
				let txns = block.transactions();

				debug!("processing block at DA height {} with {} transaction(s)", da_block.height, txns.len());

				for txn in txns {
					let aptos_transaction = bcs::from_bytes::<'_, SignedTransaction>(txn.data())
					.context("Failed to deserialize Aptos transaction")?;
					yield aptos_transaction;
				}
			}
		};

		Ok(stream)
	}
}

pub struct DaSequencerDb(DaDB);

impl DaSequencerDb {
	pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let da_db = DaDB::open(path)?;
		Ok(DaSequencerDb(da_db))
	}

	pub fn get_synced_height(&self) -> Result<u64, anyhow::Error> {
		self.0.get_synced_height()
	}
}

pub fn get_da_block_height(path: impl AsRef<Path>) -> Result<u64, anyhow::Error> {
	let db = DaSequencerDb::open(path)?;
	db.get_synced_height()
}
