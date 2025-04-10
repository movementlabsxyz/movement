use super::submit::BlobSubmitter;
use super::{
	BlockSource, CelestiaBlobData, CelestiaClientOps, CelestiaHeight, ExternalDaNotification,
};
use crate::block::SequencerBlockDigest;
use crate::error::DaSequencerError;

use celestia_rpc::Client as RpcClient;
use celestia_types::nmt::Namespace;
use tokio::sync::mpsc;
use url::Url;

use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("Celestia RPC error: {}", .0)]
	Rpc(#[from] celestia_rpc::Error),
}

#[derive(Clone)]
pub struct CelestiaClient {
	rpc_client: Arc<RpcClient>,
	notifier: mpsc::Sender<ExternalDaNotification>,
	// The sender end of the channel for the background sender task.
	digest_sender: mpsc::Sender<(SequencerBlockDigest, BlockSource)>,
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
		let (digest_sender, digest_receiver) = mpsc::channel(8);
		let blob_submitter = BlobSubmitter::new(
			Arc::clone(&rpc_client),
			celestia_namespace,
			digest_receiver,
			notifier.clone(),
		);
		tokio::spawn(blob_submitter.run());
		Ok(CelestiaClient { rpc_client, notifier, digest_sender })
	}
}

impl CelestiaClientOps for CelestiaClient {
	async fn get_blob_at_height(
		&self,
		_height: CelestiaHeight,
	) -> Result<Option<CelestiaBlobData>, DaSequencerError> {
		todo!()
	}

	async fn send_block(
		&self,
		block: SequencerBlockDigest,
		source: BlockSource,
	) -> Result<(), DaSequencerError> {
		self.digest_sender
			.send((block, source))
			.await
			.map_err(|_| DaSequencerError::SendFailure)
	}
}
