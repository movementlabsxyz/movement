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
use movement_da_light_node_verifier::VerifierOperations;
use movement_da_util::{
	blob::ir::blob::DaBlob, blob::ir::data::InnerSignedBlobV1Data, config::Config,
};

use crate::LightNodeRuntime;
use ecdsa::{
	elliptic_curve::{
		generic_array::ArrayLength,
		ops::Invert,
		point::PointCompression,
		sec1::{FromEncodedPoint, ModulusSize, ToEncodedPoint},
		subtle::CtOption,
		AffinePoint, CurveArithmetic, FieldBytesSize, PrimeCurve, Scalar,
	},
	hazmat::{DigestPrimitive, SignPrimitive, VerifyPrimitive},
	SignatureSize, SigningKey,
};

#[derive(Clone)]
pub struct LightNode<C, Da, V>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
	Da: DaOperations,
	V: VerifierOperations<DaBlob, DaBlob>,
{
	pub config: Config,
	pub signing_key: SigningKey<C>,
	pub da: Arc<Da>,
	pub verifier: Arc<V>,
}

impl<C, Da, V> Debug for LightNode<C, Da, V>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
	Da: DaOperations,
	V: VerifierOperations<DaBlob, DaBlob>,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("LightNode")
			.field("celestia_namespace", &self.config.celestia_namespace())
			.finish()
	}
}

impl<C> LightNodeRuntime for LightNode<C, DigestStoreDa<CelestiaDa>, InKnownSignersVerifier<C>>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
{
	/// Tries to create a new LightNode instance from the toml config file.
	async fn try_from_config(config: Config) -> Result<Self, anyhow::Error> {
		let signing_key_str = config.da_signing_key();
		let hex_bytes = hex::decode(signing_key_str)?;

		let signing_key = SigningKey::from_bytes(hex_bytes.as_slice().try_into()?)
			.map_err(|e| anyhow::anyhow!("Failed to create signing key: {}", e))?;

		let client = Arc::new(config.connect_celestia().await?);
		let celestia_da = CelestiaDa::new(config.celestia_namespace(), client);
		let digest_store_da = DigestStoreDa::try_new(celestia_da, config.digest_store_db_path())?;

		let verifier = Arc::new(InKnownSignersVerifier::<C>::new(config.da_signers_sec1_keys()));

		Ok(Self { config: config.clone(), da: Arc::new(digest_store_da), signing_key, verifier })
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
impl<C, Da, V> LightNodeService for LightNode<C, Da, V>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
	Da: DaOperations + Send + Sync + 'static,
	V: VerifierOperations<DaBlob, DaBlob> + Send + Sync + 'static,
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
				let blob = if height.as_u64() == 0 {
					//Heart beat blob
					// No need to verify the data are removed.
					da_blob.to_blob_heartbeat_response()
				} else {
					let verifed_blob = verifier.verify(da_blob, height.as_u64()).await.map_err(|e| tonic::Status::internal(e.to_string()))?;
					verifed_blob.into_inner().to_blob_passed_through_read_response(height.as_u64()).map_err(|e| tonic::Status::internal(e.to_string()))?
				};
				let response = StreamReadFromHeightResponse {
					blob: Some(blob)
				};
				yield response;
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
				.try_to_sign(&self.signing_key)
				.map_err(|e| tonic::Status::internal(format!("Failed to sign blob: {}", e)))?;
		}

		// * We are currently not returning any blobs in the response.
		Ok(tonic::Response::new(BatchWriteResponse { blobs: vec![] }))
	}
}
