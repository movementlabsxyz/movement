use movement_da_util::ir_blob::DaBlob;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};
use tracing::{debug, error, info, warn};

use celestia_rpc::{BlobClient, Client, HeaderClient};
use celestia_types::{nmt::Namespace, Blob as CelestiaBlob, TxConfig};

// FIXME: glob imports are bad style
use movement_da_light_node_da::{DaBlob, DaOperations};
use movement_da_light_node_proto::light_node_service_server::LightNodeService;
use movement_da_light_node_proto::*;
use movement_da_light_node_verifier::{permissioned_signers::Verifier, VerifierOperations};
use movement_da_util::{
	config::Config,
	ir_blob::{celestia::CelestiaDaBlob, InnerSignedBlobV1Data},
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
pub struct LightNode<C, Da>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
	Da: DaOperations,
{
	pub config: Config,
	pub verifier: Arc<
		Box<dyn VerifierOperations<CelestiaBlob, DaBlob> + Send + Sync>,
	>,
	pub signing_key: SigningKey<C>,
	pub da: Arc<Da>,
}

impl<C, Da> Debug for LightNode<C, Da>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
	Da: DaOperations,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("LightNode")
			.field("celestia_namespace", &self.config.celestia_namespace())
			.finish()
	}
}

impl<C, Da> LightNodeRuntime for LightNode<C, Da>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
	Da: DaOperations,
{
	/// Tries to create a new LightNode instance from the toml config file.
	async fn try_from_config(config: Config) -> Result<Self, anyhow::Error> {
		let client = Arc::new(config.connect_celestia().await?);

		let signing_key_str = config.da_signing_key();
		let hex_bytes = hex::decode(signing_key_str)?;

		let signing_key = SigningKey::from_bytes(hex_bytes.as_slice().try_into()?)
			.map_err(|e| anyhow::anyhow!("Failed to create signing key: {}", e))?;

		Ok(Self {
			config: config.clone(),
			celestia_namespace: config.celestia_namespace(),
			default_client: client.clone(),
			verifier: Arc::new(Box::new(Verifier::<C>::new(
				client,
				config.celestia_namespace(),
				config.da_signers_sec1_keys(),
			))),
			signing_key,
		})
	}

	fn try_service_address(&self) -> Result<String, anyhow::Error> {
		Ok(self.config.movement_da_light_node_service())
	}

	/// Runs background tasks for the LightNode instance.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}

impl<C, Da> LightNode<C, Da>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
	Da: DaOperations,
{
	pub fn blob_to_blob_write_response(blob: Blob) -> Result<BlobResponse, anyhow::Error> {
		Ok(BlobResponse { blob_type: Some(blob_response::BlobType::PassedThroughBlob(blob)) })
	}

	pub fn blob_to_blob_read_response(blob: Blob) -> Result<BlobResponse, anyhow::Error> {
		#[cfg(feature = "sequencer")]
		{
			Ok(BlobResponse { blob_type: Some(blob_response::BlobType::SequencedBlobBlock(blob)) })
		}

		#[cfg(not(feature = "sequencer"))]
		{
			Ok(BlobResponse { blob_type: Some(blob_response::BlobType::PassedThroughBlob(blob)) })
		}
	}
}

#[tonic::async_trait]
impl<C, Da> LightNodeService for LightNode<C, Da>
where
	C: PrimeCurve + CurveArithmetic + DigestPrimitive + PointCompression,
	Scalar<C>: Invert<Output = CtOption<Scalar<C>>> + SignPrimitive<C>,
	SignatureSize<C>: ArrayLength<u8>,
	AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C> + VerifyPrimitive<C>,
	FieldBytesSize<C>: ModulusSize,
	Da: DaOperations,
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

		let me = Arc::new(self.clone());
		let height = request.into_inner().height;

		let output = async_stream::try_stream! {

			let mut blob_stream = me.da.stream_ir_blobs_from_height(height).await.map_err(|e| tonic::Status::internal(e.to_string()))?;

			while let Some(blob) = blob_stream.next().await {
				let blob = blob.map_err(|e| tonic::Status::internal(e.to_string()))?;
				let response = StreamReadFromHeightResponse {
					blob : Some(Self::blob_to_blob_read_response(blob).map_err(|e| tonic::Status::internal(e.to_string()))?)
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
		request: tonic::Request<tonic::Streaming<StreamWriteBlobRequest>>,
	) -> std::result::Result<tonic::Response<Self::StreamWriteBlobStream>, tonic::Status> {
		unimplemented!()
	}
	/// Read blobs at a specified height.
	async fn read_at_height(
		&self,
		request: tonic::Request<ReadAtHeightRequest>,
	) -> std::result::Result<tonic::Response<ReadAtHeightResponse>, tonic::Status> {
		unimplemented!()
	}
	/// Batch read and write operations for efficiency.
	async fn batch_read(
		&self,
		request: tonic::Request<BatchReadRequest>,
	) -> std::result::Result<tonic::Response<BatchReadResponse>, tonic::Status> {
		unimplemented!()
	}

	/// Batch write blobs.
	async fn batch_write(
		&self,
		request: tonic::Request<BatchWriteRequest>,
	) -> std::result::Result<tonic::Response<BatchWriteResponse>, tonic::Status> {
		let blobs = request.into_inner().blobs;
		let mut responses = Vec::with_capacity(blobs.len());
		for data in blobs {
			let blob = self
				.da
				.submit_blob(data.data)
				.await
				.map_err(|e| tonic::Status::internal(e.to_string()))?;
			responses.push(blob);
		}

		let mut blob_responses = Vec::new();
		for blob in responses {
			blob_responses.push(
				Self::blob_to_blob_write_response(blob)
					.map_err(|e| tonic::Status::internal(e.to_string()))?,
			);
		}

		Ok(tonic::Response::new(BatchWriteResponse { blobs: blob_responses }))
	}
}
