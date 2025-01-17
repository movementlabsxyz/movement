pub mod db;

use movement_da_light_node_da::{Certificate, CertificateStream, DaError, DaOperations};
use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::cryptography::Curve;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

#[derive(Clone)]
pub struct Da<C>
where
	C: Curve + Send + Sync + Clone + 'static,
{
	/// The RocksDB instance.
	db: db::DaDb<C>,
	/// The broadcast channel for certificate notifications.
	cert_tx: Arc<broadcast::Sender<Certificate>>,
}

impl<C> Da<C>
where
	C: Curve + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
{
	/// Creates a new Da instance with the provided Celestia namespace and RPC client.
	pub fn try_new(db_path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
		let (cert_tx, _cert_rx) = broadcast::channel(100); // Create a broadcast channel with a buffer size of 100
		Ok(Self { db: db::DaDb::open(db_path)?, cert_tx: Arc::new(cert_tx) })
	}
}

impl<C> DaOperations<C> for Da<C>
where
	C: Curve + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
{
	fn submit_blob(
		&self,
		data: DaBlob<C>,
	) -> Pin<Box<dyn Future<Output = Result<(), DaError>> + Send + '_>> {
		let db = self.db.clone();
		let cert_tx = self.cert_tx.clone();

		Box::pin(async move {
			// Add the blob to the database at the next available height
			let current_height =
				db.add_blob(data).await.map_err(|e| DaError::Internal(e.to_string()))?;

			// Broadcast the certificate for the new height
			if let Err(e) = cert_tx.send(Certificate::Height(current_height)) {
				tracing::warn!(
					"Failed to broadcast certificate for height {}: {:?}",
					current_height,
					e
				);
			}

			Ok(())
		})
	}

	fn get_da_blobs_at_height(
		&self,
		height: u64,
	) -> Pin<Box<dyn Future<Output = Result<Vec<DaBlob<C>>, DaError>> + Send + '_>> {
		let db = self.db.clone();

		Box::pin(async move {
			let blob = db
				.get_blob_at_height(height)
				.await
				.map_err(|e| DaError::NonFatalBlobsAtHeight(e.into()))?;
			Ok(blob.map_or_else(Vec::new, |b| vec![b]))
		})
	}

	fn stream_certificates(
		&self,
	) -> Pin<Box<dyn Future<Output = Result<CertificateStream, DaError>> + Send + '_>> {
		let cert_rx = self.cert_tx.subscribe();

		Box::pin(async move {
			// Wrap the broadcast receiver into a stream
			let stream = BroadcastStream::new(cert_rx).filter_map(|result| match result {
				Ok(height) => Some(Ok(height)), // Pass valid heights
				Err(e) => Some(Err(DaError::Internal(e.to_string()))), // Convert to DaError
			}); // Convert to DaError

			// Box the stream and return it
			Ok(Box::pin(stream) as CertificateStream)
		})
	}
}
