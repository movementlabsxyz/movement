use crate::blob::ir::{into_da_blob, CelestiaDaBlob};
use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::{nmt::Namespace, Blob as CelestiaBlob, TxConfig};
use movement_da_light_node_da::{Certificate, CertificateStream, DaError, DaOperations};
use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::cryptography::Curve;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::error;

#[derive(Clone)]
pub struct Da<C>
where
	C: Curve + Send + Sync + Clone + 'static,
{
	/// The namespace on Celestia which the Da will use.
	celestia_namespace: Namespace,
	/// The Celestia RPC client
	default_client: Arc<Client>,
	/// The curve marker.
	__curve_marker: std::marker::PhantomData<C>,
}

impl<C> Da<C>
where
	C: Curve + Send + Sync + Clone + Serialize + 'static,
{
	/// Creates a new Da instance with the provided Celestia namespace and RPC client.
	pub fn new(celestia_namespace: Namespace, default_client: Arc<Client>) -> Self {
		Self { celestia_namespace, default_client, __curve_marker: std::marker::PhantomData }
	}

	/// Creates a new signed blob instance with the provided DaBlob data.
	pub fn create_new_celestia_blob(&self, data: DaBlob<C>) -> Result<CelestiaBlob, anyhow::Error> {
		// create the celestia blob
		CelestiaDaBlob(data.into(), self.celestia_namespace.clone()).try_into()
	}

	/// Submits a CelestiaBlob to the Celestia node.
	pub async fn submit_celestia_blob(&self, blob: CelestiaBlob) -> Result<u64, anyhow::Error> {
		let config = TxConfig::default();
		// config.with_gas(2);
		let height = self.default_client.blob_submit(&[blob], config).await.map_err(|e| {
			error!(error = %e, "failed to submit the blob");
			anyhow::anyhow!("Failed submitting the blob: {}", e)
		})?;

		Ok(height)
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
		Box::pin(async move {
			// create the blob
			let celestia_blob = self
				.create_new_celestia_blob(data)
				.map_err(|e| DaError::Internal("failed to create celestia blob".to_string()))?;

			// submit the blob to the celestia node
			self.submit_celestia_blob(celestia_blob)
				.await
				.map_err(|e| DaError::Internal("failed to submit celestia blob".to_string()))?;

			Ok(())
		})
	}

	fn get_da_blobs_at_height(
		&self,
		height: u64,
	) -> Pin<Box<dyn Future<Output = Result<Vec<DaBlob<C>>, DaError>> + Send + '_>> {
		Box::pin(async move {
			let height = if height == 0 { 1 } else { height };

			match self.default_client.blob_get_all(height, &[self.celestia_namespace]).await {
				// todo: lots more pattern matching here
				Err(e) => {
					error!(error = %e, "failed to get blobs at height {height}");
					Err(DaError::NonFatalBlobsAtHeight(
						format!("failed to get blobs at height {height}").into(),
					))
				}
				Ok(blobs) => {
					let blobs = blobs.unwrap_or_default();
					let mut da_blobs = Vec::new();

					for blob in blobs {
						let da_blob = into_da_blob(blob).map_err(|e| {
							DaError::NonFatalBlobsAtHeight(
								format!("failed to convert blob: {e}").into(),
							)
						})?;
						da_blobs.push(da_blob);
					}

					Ok(da_blobs)
				}
			}
		})
	}

	fn stream_certificates(
		&self,
	) -> Pin<Box<dyn Future<Output = Result<CertificateStream, DaError>> + Send + '_>> {
		let me = self.clone();
		Box::pin(async move {
			let mut subscription = me.default_client.header_subscribe().await.map_err(|e| {
				DaError::Certificate("failed to subscribe to headers".to_string().into())
			})?;
			let stream = async_stream::try_stream! {

				while let Some(header_res) = subscription.next().await {

					let header = header_res.map_err(|e| {
						DaError::NonFatalCertificate(e.into())
					})?;
					let height = header.height().into();

					yield Certificate::Height(height);

				}
			};
			Ok(Box::pin(stream) as CertificateStream)
		})
	}
}
