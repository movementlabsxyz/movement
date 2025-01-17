use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::cryptography::Curve;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::path::Path;
use std::sync::Arc;

mod column_families {
	pub const DIGESTED_BLOBS: &str = "digested_blobs";
}
use column_families::*;

/// Simple data store for locally recorded DA events.
///
/// An async access API is provided to avoid blocking async tasks.
/// The methods must be executed in the context of a Tokio runtime.
#[derive(Clone, Debug)]
pub struct DaDB<C>
where
	C: Curve + Send + Sync + Clone + 'static,
{
	inner: Arc<DB>,
	__curve_marker: std::marker::PhantomData<C>,
}

impl<C> DaDB<C>
where
	C: Curve + Send + Sync + Clone + 'static,
{
	pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
		let mut options = Options::default();
		options.create_if_missing(true);
		options.create_missing_column_families(true);

		let synced_height = ColumnFamilyDescriptor::new(DIGESTED_BLOBS, Options::default());

		let db = DB::open_cf_descriptors(&options, path, vec![synced_height])
			.map_err(|e| anyhow::anyhow!("Failed to open DA DB: {:?}", e))?;
		Ok(Self { inner: Arc::new(db), __curve_marker: std::marker::PhantomData })
	}

	/// Adds a digested blob to the database.
	pub async fn add_digested_blob(
		&self,
		id: Vec<u8>,
		blob: DaBlob<C>,
	) -> Result<(), anyhow::Error> {
		let da_db = self.inner.clone();
		tokio::task::spawn_blocking(move || {
			let cf = da_db
				.cf_handle(DIGESTED_BLOBS)
				.ok_or(anyhow::anyhow!("No digested_blobs column family"))?;
			let blob = bcs::to_bytes(&blob)
				.map_err(|e| anyhow::anyhow!("Failed to serialize digested blob: {:?}", e))?;
			da_db
				.put_cf(&cf, id.clone(), blob)
				.map_err(|e| anyhow::anyhow!("Failed to add digested blob: {:?}", e))
		})
		.await??;
		Ok(())
	}

	/// Gets a digested blob from the database.
	pub async fn get_digested_blob(&self, id: Vec<u8>) -> Result<Option<DaBlob<C>>, anyhow::Error> {
		let da_db = self.inner.clone();
		let blob = tokio::task::spawn_blocking(move || {
			let cf = da_db
				.cf_handle(DIGESTED_BLOBS)
				.ok_or(anyhow::anyhow!("No digested_blobs column family"))?;
			let blob = da_db
				.get_cf(&cf, id)
				.map_err(|e| anyhow::anyhow!("Failed to get digested blob: {:?}", e))?;
			let blob = match blob {
				Some(blob) => Some(bcs::from_bytes(&blob).map_err(|e| {
					anyhow::anyhow!("Failed to deserialize digested blob: {:?}", e)
				})?),
				None => None,
			};
			Ok::<Option<DaBlob<C>>, anyhow::Error>(blob)
		})
		.await??;
		Ok(blob)
	}
}
