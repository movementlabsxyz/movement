use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::cryptography::Curve;
use rocksdb::{ColumnFamilyDescriptor, Options, TransactionDB, TransactionDBOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::task;

mod column_families {
	pub const BLOBS: &str = "blobs";
	pub const LAST_HEIGHT: &str = "last_height";
}
use column_families::*;

/// Simple data store for locally recorded DA events with height tracking.
///
/// Methods are designed to work within a Tokio runtime.
#[derive(Clone)]
pub struct DaDb<C>
where
	C: Curve,
{
	inner: Arc<TransactionDB>,
	__curve_marker: std::marker::PhantomData<C>,
}

impl<C> DaDb<C>
where
	C: Curve + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
{
	/// Opens or creates the transactional database at the given path.
	pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let mut options = Options::default();
		options.create_if_missing(true);
		options.create_missing_column_families(true);

		let blobs_cf = ColumnFamilyDescriptor::new(BLOBS, Options::default());
		let last_height_cf = ColumnFamilyDescriptor::new(LAST_HEIGHT, Options::default());

		let db = TransactionDB::open_cf_descriptors(
			&options,
			&TransactionDBOptions::default(),
			path,
			vec![blobs_cf, last_height_cf],
		)
		.map_err(|e| anyhow::anyhow!("Failed to open transactional database: {:?}", e))?;

		Ok(Self { inner: Arc::new(db), __curve_marker: std::marker::PhantomData })
	}

	/// Adds a blob at the next height, using a transaction to ensure consistency.
	pub async fn add_blob(&self, blob: DaBlob<C>) -> anyhow::Result<u64> {
		let db = self.inner.clone();

		task::spawn_blocking(move || {
			let transaction = db.transaction();

			// Retrieve the current height
			let last_height_cf = db
				.cf_handle(LAST_HEIGHT)
				.ok_or_else(|| anyhow::anyhow!("Missing column family: {}", LAST_HEIGHT))?;

			let current_height: u64 = transaction
				.get_cf(&last_height_cf, b"last_height")
				.unwrap_or_else(|_| Some(vec![0]))
				.and_then(|v| String::from_utf8(v).ok())
				.and_then(|s| s.parse::<u64>().ok())
				.unwrap_or(0);

			// Serialize the blob
			let blobs_cf = db
				.cf_handle(BLOBS)
				.ok_or_else(|| anyhow::anyhow!("Missing column family: {}", BLOBS))?;

			let blob_bytes = bcs::to_bytes(&blob)
				.map_err(|e| anyhow::anyhow!("Failed to serialize blob: {:?}", e))?;

			// Store the blob at the current height
			transaction
				.put_cf(&blobs_cf, current_height.to_be_bytes(), blob_bytes)
				.map_err(|e| anyhow::anyhow!("Failed to store blob: {:?}", e))?;

			// Update the height
			let next_height = current_height + 1;
			transaction
				.put_cf(&last_height_cf, b"last_height", next_height.to_string().as_bytes())
				.map_err(|e| anyhow::anyhow!("Failed to update height: {:?}", e))?;

			// Commit the transaction
			transaction
				.commit()
				.map_err(|e| anyhow::anyhow!("Transaction failed: {:?}", e))?;

			Ok(current_height)
		})
		.await?
	}

	/// Retrieves a blob at the specified height.
	pub async fn get_blob_at_height(&self, height: u64) -> anyhow::Result<Option<DaBlob<C>>> {
		let db = self.inner.clone();

		task::spawn_blocking(move || {
			let blobs_cf = db
				.cf_handle(BLOBS)
				.ok_or_else(|| anyhow::anyhow!("Missing column family: {}", BLOBS))?;

			match db.get_cf(&blobs_cf, height.to_be_bytes()) {
				Ok(Some(blob_bytes)) => {
					let blob = bcs::from_bytes(&blob_bytes)
						.map_err(|e| anyhow::anyhow!("Failed to deserialize blob: {:?}", e))?;
					Ok(Some(blob))
				}
				Ok(None) => Ok(None),
				Err(e) => Err(anyhow::anyhow!("Failed to retrieve blob: {:?}", e)),
			}
		})
		.await?
	}

	/// Gets the current height.
	pub async fn current_height(&self) -> anyhow::Result<u64> {
		let db = self.inner.clone();

		task::spawn_blocking(move || {
			let last_height_cf = db
				.cf_handle(LAST_HEIGHT)
				.ok_or_else(|| anyhow::anyhow!("Missing column family: {}", LAST_HEIGHT))?;

			let current_height: u64 = db
				.get_cf(&last_height_cf, b"last_height")
				.unwrap_or_else(|_| Some(vec![0]))
				.and_then(|v| String::from_utf8(v).ok())
				.and_then(|s| s.parse::<u64>().ok())
				.unwrap_or(0);

			Ok(current_height)
		})
		.await?
	}
}
