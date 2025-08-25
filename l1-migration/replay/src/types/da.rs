use movement_da_sequencer_client::DaSequencerClient as _;
use movement_da_sequencer_client::{GrpcDaSequencerClient, StreamReadBlockFromHeight};
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use rocksdb::{ColumnFamilyDescriptor, DB};
use std::cell::Cell;
use std::path::Path;

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
		let mut client = self.0.take().expect("Da-sequencer client should be always avaialable");
		let request = StreamReadFromHeightRequest { height: block_height };
		let result = client.stream_read_from_height(request).await;
		self.0.set(Some(client));
		let (blocks, _) = result?;
		Ok(blocks)
	}
}

const SYNCED_HEIGHT: &str = "synced_height";

pub struct DaSequencerDb(DB);

impl DaSequencerDb {
	pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let options = rocksdb::Options::default();
		let synced_height = ColumnFamilyDescriptor::new(SYNCED_HEIGHT, rocksdb::Options::default());
		let db = DB::open_cf_descriptors(&options, path, vec![synced_height])
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
