use rocksdb::{ColumnFamilyDescriptor, Options, DB};

use std::path::Path;
use std::sync::Arc;

mod column_families {
	pub const EXECUTED_BLOCKS: &str = "executed_blocks";
	pub const SYNCED_HEIGHT: &str = "synced_height";
}
use column_families::*;

/// Simple data store for locally recorded DA events.
///
/// An async access API is provided to avoid blocking async tasks.
/// The methods must be executed in the context of a Tokio runtime.
#[derive(Clone, Debug)]
pub struct DaDB {
	inner: Arc<DB>,
}

impl DaDB {
	pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let mut options = Options::default();
		options.create_if_missing(true);
		options.create_missing_column_families(true);

		let synced_height = ColumnFamilyDescriptor::new(SYNCED_HEIGHT, Options::default());
		let executed_blocks = ColumnFamilyDescriptor::new(EXECUTED_BLOCKS, Options::default());

		let db = DB::open_cf_descriptors(&options, path, vec![synced_height, executed_blocks])
			.map_err(|e| anyhow::anyhow!("Failed to open DA DB: {:?}", e))?;
		Ok(Self { inner: Arc::new(db) })
	}

	pub fn add_executed_block(&self, id: Vec<u8>) -> Result<(), anyhow::Error> {
		let da_db = self.inner.clone();
		let cf = da_db
			.cf_handle(EXECUTED_BLOCKS)
			.ok_or(anyhow::anyhow!("No executed_blocks column family"))?;
		da_db
			.put_cf(&cf, id.clone(), id)
			.map_err(|e| anyhow::anyhow!("Failed to add executed block: {:?}", e))?;
		Ok(())
	}

	pub fn has_executed_block(&self, id: Vec<u8>) -> Result<bool, anyhow::Error> {
		let da_db = self.inner.clone();
		let id = {
			let cf = da_db
				.cf_handle(EXECUTED_BLOCKS)
				.ok_or(anyhow::anyhow!("No executed_blocks column family"))?;
			da_db
				.get_cf(&cf, id)
				.map_err(|e| anyhow::anyhow!("Failed to get executed block: {:?}", e))
		}?;
		Ok(id.is_some())
	}

	pub fn set_synced_height(&self, height: u64) -> Result<(), anyhow::Error> {
		// This is heavy for this purpose, but progressively the contents of the DA DB will be used for more things
		let da_db = self.inner.clone();
		let cf = da_db
			.cf_handle(SYNCED_HEIGHT)
			.ok_or(anyhow::anyhow!("No synced_height column family"))?;
		let height_str = serde_json::to_string(&height)
			.map_err(|e| anyhow::anyhow!("Failed to serialize synced height: {:?}", e))?;
		da_db
			.put_cf(&cf, "synced_height", height_str)
			.map_err(|e| anyhow::anyhow!("Failed to set synced height_str: {:?}", e))?;
		Ok(())
	}

	/// Get the synced height marker stored in the database.
	pub fn get_synced_height(&self) -> Result<u64, anyhow::Error> {
		// This is heavy for this purpose, but progressively the contents of the DA DB will be used for more things
		let da_db = self.inner.clone();
		let height = {
			let cf = da_db
				.cf_handle(SYNCED_HEIGHT)
				.ok_or(anyhow::anyhow!("No synced_height column family"))?;
			let height = da_db
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

	/// Set the initial value of the synced height, unless a value is stored.
	pub async fn initialize_synced_height(&self, min_height: u64) -> Result<(), anyhow::Error> {
		// This is heavy for this purpose, but progressively the contents of the DA DB will be used for more things
		let da_db = self.inner.clone();
		let cf = da_db
			.cf_handle(SYNCED_HEIGHT)
			.ok_or(anyhow::anyhow!("No synced_height column family"))?;
		let height = da_db
			.get_cf(&cf, "synced_height")
			.map_err(|e| anyhow::anyhow!("Failed to get synced height: {:?}", e))?;
		if height.is_none() {
			let height = serde_json::to_string(&min_height)
				.map_err(|e| anyhow::anyhow!("Failed to serialize synced height: {:?}", e))?;
			da_db
				.put_cf(&cf, "synced_height", height)
				.map_err(|e| anyhow::anyhow!("Failed to set synced height: {:?}", e))?;
		}
		Ok(())
	}
}
