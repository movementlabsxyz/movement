use anyhow::Context;
use aptos_types::transaction::SignedTransaction;
use futures::Stream;
use futures::TryStreamExt;
use movement_da_sequencer_client::DaSequencerClient as _;
use movement_da_sequencer_client::{GrpcDaSequencerClient, StreamReadBlockFromHeight};
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use rocksdb::{ColumnFamilyDescriptor, DB};
use std::cell::Cell;
use std::path::Path;
use tracing::info;

pub struct DaSequencerClient(Cell<Option<GrpcDaSequencerClient>>);

impl DaSequencerClient {
	pub async fn try_connect(url: &str) -> Result<Self, anyhow::Error> {
		let client = GrpcDaSequencerClient::try_connect(
			&url.parse()
				.map_err(|e| anyhow::anyhow!("Failed to parse DA-Sequencer url: {}", e))?,
			10,
		)
		.await?;
		Ok(Self(Cell::new(Some(client))))
	}

	pub async fn stream_blocks_from_height(
		&self,
		block_height: u64,
	) -> Result<StreamReadBlockFromHeight, anyhow::Error> {
		let Some(mut client) = self.0.take() else { unreachable!() };
		let request = StreamReadFromHeightRequest { height: block_height };
		let result = client.stream_read_from_height(request).await;
		self.0.set(Some(client));
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

				info!("processing block at DA height {} with {} transaction(s)", da_block.height, txns.len());

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

const SYNCED_HEIGHT: &str = "synced_height";
pub const EXECUTED_BLOCKS: &str = "executed_blocks";

pub struct DaSequencerDb(DB);

impl DaSequencerDb {
	pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let options = rocksdb::Options::default();
		let synced_height = ColumnFamilyDescriptor::new(SYNCED_HEIGHT, rocksdb::Options::default());
		let executed_blocks =
			ColumnFamilyDescriptor::new(EXECUTED_BLOCKS, rocksdb::Options::default());
		let db = DB::open_cf_descriptors(&options, path, vec![synced_height, executed_blocks])
			.map_err(|e| anyhow::anyhow!("Failed to open DA-Sequencer DB: {:?}", e))?;

		Ok(Self(db))
	}

	/// Get the synced height marker stored in the database.
	pub fn get_synced_height(&self) -> Result<u64, anyhow::Error> {
		// This is heavy for this purpose, but progressively the contents of the DA DB will be used for more things
		let height = {
			let cf = self
				.0
				.cf_handle(SYNCED_HEIGHT)
				.ok_or(anyhow::anyhow!("No synced_height column family"))?;
			let height = self
				.0
				.get_cf(&cf, "synced_height")
				.map_err(|e| anyhow::anyhow!("Failed to get synced height: {:?}", e))?;
			let height = match height {
				Some(height) => serde_json::from_slice(&height)
					.map_err(|e| anyhow::anyhow!("Failed to deserialize synced height: {:?}", e))?,
				None => 0,
			};
			Ok::<u64, anyhow::Error>(height)
		}?;
		Ok(height)
	}
}
