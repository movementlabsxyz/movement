use super::submit::BlobSubmitter;
use super::{BlockSource, CelestiaBlob, CelestiaClientOps, CelestiaHeight, ExternalDaNotification};
use crate::error::DaSequencerError;
use movement_types::block;

use celestia_rpc::{BlobClient as _, Client as RpcClient};
use celestia_types::nmt::Namespace;
use tokio::sync::mpsc;
use url::Url;

use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Celestia RPC error: {}", .0)]
	Rpc(#[from] celestia_rpc::Error),
}

#[derive(Clone)]
pub struct CelestiaClient {
	rpc_client: Arc<RpcClient>,
	celestia_namespace: Namespace,
	_notifier: mpsc::Sender<ExternalDaNotification>,
	// The sender end of the channel for the background sender task.
	id_sender: mpsc::Sender<(block::Id, BlockSource)>,
}

impl CelestiaClient {
	/// Create the Celestia client and all async process to manage celestia access.
	pub async fn new(
		connection_url: Url,
		auth_token: Option<&str>,
		celestia_namespace: Namespace,
		notifier: mpsc::Sender<ExternalDaNotification>,
	) -> Result<Self, Error> {
		let rpc_client = RpcClient::new(&connection_url.to_string(), auth_token).await?;
		let rpc_client = Arc::new(rpc_client);
		let (id_sender, id_receiver) = mpsc::channel(8);
		let blob_submitter = BlobSubmitter::new(
			Arc::clone(&rpc_client),
			celestia_namespace.clone(),
			id_receiver,
			notifier.clone(),
		);
		tokio::spawn(blob_submitter.run());
		Ok(CelestiaClient { rpc_client, celestia_namespace, _notifier: notifier, id_sender })
	}
}

impl CelestiaClientOps for CelestiaClient {
	async fn get_blob_at_height(
		&self,
		height: CelestiaHeight,
	) -> Result<Option<CelestiaBlob>, DaSequencerError> {
		match self
			.rpc_client
			.blob_get_all(height.into(), &[self.celestia_namespace])
			.await
			.map_err(|e| DaSequencerError::Rpc(e.to_string()))?
		{
			None => Ok(None),
			Some(blobs) => {
				let mut iter = blobs.into_iter();
				if let Some(rpc_blob) = iter.next() {
					let mut aggregate_blob = CelestiaBlob::try_from_rpc(rpc_blob)?;
					while let Some(rpc_blob) = iter.next() {
						let blob = CelestiaBlob::try_from_rpc(rpc_blob)?;
						aggregate_blob.merge(blob);
					}
					Ok(Some(aggregate_blob))
				} else {
					Ok(None)
				}
			}
		}
	}

	async fn send_block(
		&self,
		block_id: block::Id,
		source: BlockSource,
	) -> Result<(), DaSequencerError> {
		self.id_sender
			.send((block_id, source))
			.await
			.map_err(|_| DaSequencerError::SendFailure)
	}
}
