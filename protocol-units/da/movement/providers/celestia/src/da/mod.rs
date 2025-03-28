use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::{nmt::Namespace, Blob as CelestiaBlob};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error};

use crate::blob::ir::{into_da_blob, CelestiaDaBlob};
use movement_da_light_node_da::{Certificate, CertificateStream, DaError, DaOperations};
use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::cryptography::Curve;

#[derive(Clone)]
pub struct Da<C>
where
	C: Curve + Send + Sync + Clone + 'static,
{
	/// The namespace on Celestia which the Da will use.
	celestia_namespace: Namespace,
	/// The Celestia RPC client
	default_client: Arc<Client>,
	// Sender end of the channel for the background sender task.
	blob_sender: mpsc::Sender<CelestiaBlob>,
	/// The curve marker.
	__curve_marker: std::marker::PhantomData<C>,
}

impl<C> Da<C>
where
	C: Curve + Send + Sync + Clone + Serialize + 'static,
{
	/// Creates a new Da instance with the provided Celestia namespace and RPC client.
	pub fn new(celestia_namespace: Namespace, default_client: Arc<Client>) -> Self {
		let (blob_sender, blob_receiver) = mpsc::channel(8);
		let blob_submitter = BlobSubmitter::new(Arc::clone(&default_client), blob_receiver);
		tokio::spawn(blob_submitter.run());
		Self {
			celestia_namespace,
			default_client,
			blob_sender,
			__curve_marker: std::marker::PhantomData,
		}
	}

	/// Creates a new signed blob instance with the provided DaBlob data.
	pub fn create_new_celestia_blob(&self, data: DaBlob<C>) -> Result<CelestiaBlob, anyhow::Error> {
		// create the celestia blob
		CelestiaDaBlob(data.into(), self.celestia_namespace.clone()).try_into()
	}
}

impl<C> DaOperations<C> for Da<C>
where
	C: Curve + Send + Sync + Clone + Serialize + Debug + for<'de> Deserialize<'de> + 'static,
{
	async fn submit_blob(&self, data: DaBlob<C>) -> Result<(), DaError> {
		debug!("queuing blob to submit to Celestia: {:?}", data);

		// create the blob
		let celestia_blob = self
			.create_new_celestia_blob(data)
			.map_err(|e| DaError::Internal(format!("failed to create Celestia blob: {e}")))?;

		debug!("created celestia blob {:?}", celestia_blob);

		// submit the blob to the celestia node
		self.blob_sender
			.send(celestia_blob)
			.await
			.map_err(|e| DaError::Internal(format!("failed to submit Celestia blob: {e}")))?;

		Ok(())
	}

	async fn get_da_blobs_at_height(&self, height: u64) -> Result<Vec<DaBlob<C>>, DaError> {
		debug!("getting blobs at height {height}");
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
					debug!("got blob {da_blob:?}");
					da_blobs.push(da_blob);
				}

				Ok(da_blobs)
			}
		}
	}

	fn stream_certificates(
		&self,
	) -> impl Future<Output = Result<CertificateStream, DaError>> + Send {
		let me = self.clone();
		async move {
			let mut subscription = me.default_client.header_subscribe().await.map_err(|e| {
				DaError::Certificate(format!("failed to subscribe to headers :{e}").into())
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
		}
	}
}
