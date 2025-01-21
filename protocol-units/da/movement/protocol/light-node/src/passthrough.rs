use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};
use tracing::info;

// FIXME: glob imports are bad style
use movement_da_light_node_celestia::da::Da as CelestiaDa;
use movement_da_light_node_da::DaOperations;
use movement_da_light_node_digest_store::da::Da as DigestStoreDa;
use movement_da_light_node_proto::light_node_service_server::LightNodeService;
use movement_da_light_node_proto::*;
use movement_da_light_node_verifier::signed::InKnownSignersVerifier;
use movement_da_light_node_verifier::{Error as VerifierError, VerifierOperations};
use movement_da_util::{
	blob::ir::blob::DaBlob, blob::ir::data::InnerSignedBlobV1Data, config::Config,
};

use crate::LightNodeRuntime;
use movement_da_light_node_signer::Signer;
use movement_da_util::LoadSigner;
use movement_signer::cryptography::secp256k1::Secp256k1;
use movement_signer::{cryptography::Curve, Digester, Signing, Verify};
use movement_signer_loader::LoadedSigner;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct LightNode<O, C, Da, V>
where
	O: Signing<C> + Send + Sync + Clone,
	C: Curve
		+ Verify<C>
		+ Digester<C>
		+ Send
		+ Sync
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ Clone
		+ 'static,
	Da: DaOperations<C>,
	V: VerifierOperations<DaBlob<C>, DaBlob<C>>,
{
	pub config: Config,
	pub signer: Arc<Signer<O, C>>,
	pub da: Arc<Da>,
	pub verifier: Arc<V>,
}

impl<O, C, Da, V> Debug for LightNode<O, C, Da, V>
where
	O: Signing<C> + Send + Sync + Clone,
	C: Curve
		+ Verify<C>
		+ Digester<C>
		+ Send
		+ Sync
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ Clone
		+ 'static,
	Da: DaOperations<C>,
	V: VerifierOperations<DaBlob<C>, DaBlob<C>>,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("LightNode")
			.field("celestia_namespace", &self.config.celestia_namespace())
			.finish()
	}
}

impl LightNodeRuntime
	for LightNode<
		LoadedSigner<Secp256k1>,
		Secp256k1,
		DigestStoreDa<Secp256k1, CelestiaDa<Secp256k1>>,
		InKnownSignersVerifier<Secp256k1>,
	>
{
	/// Tries to create a new LightNode instance from the toml config file.
	async fn try_from_config(config: Config) -> Result<Self, anyhow::Error> {
		let loaded_signer: LoadedSigner<Secp256k1> =
			<Config as LoadSigner<Secp256k1>>::da_signer(&config).await?;
		let signer = Arc::new(Signer::new(loaded_signer));

		let client = Arc::new(config.connect_celestia().await?);
		let celestia_da = CelestiaDa::new(config.celestia_namespace(), client);
		let digest_store_da = DigestStoreDa::try_new(celestia_da, config.digest_store_db_path())?;

		let verifier =
			Arc::new(InKnownSignersVerifier::<Secp256k1>::new(config.da_signers_sec1_keys()));

		Ok(Self { config: config.clone(), da: Arc::new(digest_store_da), signer, verifier })
	}

	fn try_service_address(&self) -> Result<String, anyhow::Error> {
		Ok(self.config.movement_da_light_node_service())
	}

	/// Runs background tasks for the LightNode instance.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}

#[tonic::async_trait]
impl<O, C, Da, V> LightNodeService for LightNode<O, C, Da, V>
where
	O: Signing<C> + Send + Sync + Clone + 'static,
	C: Curve
		+ Verify<C>
		+ Digester<C>
		+ Send
		+ Sync
		+ Serialize
		+ for<'de> Deserialize<'de>
		+ Clone
		+ 'static,
	Da: DaOperations<C> + 'static,
	V: VerifierOperations<DaBlob<C>, DaBlob<C>> + Send + Sync + 'static,
{
	/// Server streaming response type for the StreamReadFromHeight method.
	type StreamReadFromHeightStream = std::pin::Pin<
		Box<
			dyn Stream<Item = Result<StreamReadFromHeightResponse, tonic::Status>> + Send + 'static,
		>,
	>;

	/// Stream blobs from a specified height or from the latest height.
	async fn stream_read_from_height(
		&self,
		request: tonic::Request<StreamReadFromHeightRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadFromHeightStream>, tonic::Status> {
		info!("Stream read from height request: {:?}", request);

		let da = self.da.clone();
		let verifier = self.verifier.clone();
		let height = request.into_inner().height;

		let output = async_stream::try_stream! {

			let mut blob_stream = da.stream_da_blobs_from_height(height).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

			while let Some(blob) = blob_stream.next().await {
				let (height, da_blob) = blob.map_err(|e| tonic::Status::internal(e.to_string()))?;
				match verifier.verify(da_blob, height.as_u64()).await {
					Ok(verifed_blob) => {
						let blob = verifed_blob.into_inner().to_blob_passed_through_read_response(height.as_u64()).map_err(|e| tonic::Status::internal(e.to_string()))?;
						let response = StreamReadFromHeightResponse {
							blob: Some(blob)
						};
						yield response;
					},
					Err(VerifierError::Validation(e)) => {
						info!("Failed to verify blob: {}", e);
					},
					Err(VerifierError::Internal(e)) => {
						Err(tonic::Status::internal(e.to_string()))?;
					}
				}
			}

			info!("Stream read from height closed for height: {}", height);

		};

		Ok(tonic::Response::new(Box::pin(output) as Self::StreamReadFromHeightStream))
	}

	/// Server streaming response type for the StreamReadLatest method.
	type StreamReadLatestStream = std::pin::Pin<
		Box<dyn Stream<Item = Result<StreamReadLatestResponse, tonic::Status>> + Send + 'static>,
	>;

	/// Stream the latest blobs.
	async fn stream_read_latest(
		&self,
		_request: tonic::Request<StreamReadLatestRequest>,
	) -> std::result::Result<tonic::Response<Self::StreamReadLatestStream>, tonic::Status> {
		unimplemented!()
	}
	/// Server streaming response type for the StreamWriteCelestiaBlob method.
	type StreamWriteBlobStream = std::pin::Pin<
		Box<dyn Stream<Item = Result<StreamWriteBlobResponse, tonic::Status>> + Send + 'static>,
	>;
	/// Stream blobs out, either individually or in batches.
	async fn stream_write_blob(
		&self,
		_request: tonic::Request<tonic::Streaming<StreamWriteBlobRequest>>,
	) -> std::result::Result<tonic::Response<Self::StreamWriteBlobStream>, tonic::Status> {
		unimplemented!()
	}
	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		_request: tonic::Request<ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		unimplemented!()
	}
	/// Batch read and write operations for efficiency.
	async fn batch_read(
		&self,
		_request: tonic::Request<BatchReadRequest>,
	) -> std::result::Result<tonic::Response<BatchReadResponse>, tonic::Status> {
		unimplemented!()
	}

	/// Batch write blobs.
	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		let blobs = request.into_inner().blobs;
		for data in blobs {
			let blob = InnerSignedBlobV1Data::now(data.data)
				.map_err(|e| tonic::Status::internal(format!("Failed to create blob data: {}", e)))?
				.try_to_sign(&self.signer)
				.await
				.map_err(|e| tonic::Status::internal(format!("Failed to sign blob: {}", e)))?;
			self.da
				.submit_blob(blob.into())
				.await
				.map_err(|e| tonic::Status::internal(e.to_string()))?;
		}

		// * We are currently not returning any blobs in the response.
		Ok(tonic::Response::new(BatchWriteResponse { blobs: vec![] }))
	}
}
