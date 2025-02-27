pub mod db;

use movement_da_light_node_da::{CertificateStream, DaError, DaOperations};
use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::cryptography::Curve;
use movement_signer::{Digester, Verify};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
pub struct Da<C, D>
where
	C: Curve + Send + Sync + Clone + 'static + std::fmt::Debug,
	D: DaOperations<C>,
{
	/// The namespace on Celestia which the Da will use.
	inner: Arc<D>,
	/// The RocksDB instance.
	db: db::DaDB<C>,
	/// The curve marker.
	_curve_marker: std::marker::PhantomData<C>,
}

impl<C, D> Da<C, D>
where
	C: Curve
		+ Send
		+ Sync
		+ Clone
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ 'static
		+ std::fmt::Debug,
	D: DaOperations<C>,
{
	/// Creates a new Da instance with the provided Celestia namespace and RPC client.
	pub fn try_new(inner: D, db_path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
		Ok(Self {
			inner: Arc::new(inner),
			db: db::DaDB::open(db_path)?,
			_curve_marker: std::marker::PhantomData,
		})
	}
}

impl<C, D> DaOperations<C> for Da<C, D>
where
	C: Curve
		+ Verify<C>
		+ Digester<C>
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ Send
		+ Sync
		+ Clone
		+ 'static
		+ std::fmt::Debug,
	D: DaOperations<C>,
{
	fn submit_blob(
		&self,
		data: DaBlob<C>,
	) -> Pin<Box<dyn Future<Output = Result<(), DaError>> + Send + '_>> {
		Box::pin(async move {
			// get the digest
			let digest = data.id().to_vec();

			// store the digested blob
			self.db
				.add_digested_blob(digest.clone(), data)
				.await
				.map_err(|e| DaError::Internal(format!("failed to store digested blob: {}", e)))?;

			// create a digest blob
			let digest_blob = DaBlob::DigestV1(digest);

			// submit the digest blob to the inner da
			self.inner.submit_blob(digest_blob).await?;

			Ok(())
		})
	}

	fn get_da_blobs_at_height(
		&self,
		height: u64,
	) -> Pin<Box<dyn Future<Output = Result<Vec<DaBlob<C>>, DaError>> + Send + '_>> {
		Box::pin(async move {
			// get the blobs from the inner da
			let inner_blobs = self.inner.get_da_blobs_at_height(height).await?;

			let mut blobs = Vec::new();
			for inner_blob in inner_blobs {
				if let Some(blob) =
					self.db.get_digested_blob(inner_blob.id().to_vec()).await.map_err(|e| {
						DaError::NonFatalBlobsAtHeight(
							format!("failed to get digested blob: {}", e).into(),
						)
					})? {
					blobs.push(blob);
				}
			}

			Ok(blobs)
		})
	}

	fn stream_certificates(
		&self,
	) -> Pin<Box<dyn Future<Output = Result<CertificateStream, DaError>> + Send + '_>> {
		// simply pass through to streaming the underlying DA certificates
		self.inner.stream_certificates()
	}
}
