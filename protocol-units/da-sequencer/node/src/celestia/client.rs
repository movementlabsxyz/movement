use super::submit::BlobSubmitter;
use super::{BlockSource, CelestiaBlob, CelestiaClientOps, CelestiaHeight, ExternalDaNotification};
use crate::error::DaSequencerError;
use movement_signer_loader::identifiers::SignerIdentifier;
use movement_signer_loader::Load;
use movement_types::block;

use celestia_rpc::Client as RpcClient;
use celestia_types::nmt::Namespace;
use tokio::sync::mpsc;
use url::Url;

use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Celestia RPC error: {}", .0)]
	Rpc(#[from] celestia_rpc::Error),
	#[error("failed to initialize remote signer: {}", .0)]
	SignerLoad(movement_signer_loader::LoaderError),
}

#[derive(Clone)]
pub struct CelestiaClient {
	_rpc_client: Arc<RpcClient>,
	_notifier: mpsc::Sender<ExternalDaNotification>,
	// The sender end of the channel for the background sender task.
	id_sender: mpsc::Sender<(block::Id, BlockSource)>,
}

impl CelestiaClient {
	/// Create the Celestia client and all async process to manage celestia access.
	pub async fn new(
		connection_url: Url,
		auth_token: Option<&str>,
		signer_identifier: SignerIdentifier,
		celestia_namespace: Namespace,
		notifier: mpsc::Sender<ExternalDaNotification>,
	) -> Result<Self, Error> {
		let (rpc_client, signer) = tokio::try_join!(
			async {
				RpcClient::new(&connection_url.to_string(), auth_token)
					.await
					.map_err(Error::Rpc)
			},
			async { signer_identifier.load().await.map_err(Error::SignerLoad) },
		)?;
		let rpc_client = Arc::new(rpc_client);
		let (digest_sender, digest_receiver) = mpsc::channel(8);
		let blob_submitter = BlobSubmitter::new(
			Arc::clone(&rpc_client),
			celestia_namespace,
			signer,
			digest_receiver,
			notifier.clone(),
		);
		tokio::spawn(blob_submitter.run());
		Ok(CelestiaClient {
			_rpc_client: rpc_client,
			_notifier: notifier,
			id_sender: digest_sender,
		})
	}
}

impl CelestiaClientOps for CelestiaClient {
	async fn get_blob_at_height(
		&self,
		_height: CelestiaHeight,
	) -> Result<Option<CelestiaBlob>, DaSequencerError> {
		todo!()
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
