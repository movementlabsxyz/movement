pub mod fifo;

use movement_da_util::blob::ir::blob::IntermediateBlobRepresentation;
use std::error;
use std::future::Future;
use tokio_stream::{Stream, StreamExt};

pub type CertificateStream =
	std::pin::Pin<Box<dyn Stream<Item = Result<Certificate, DaError>> + Send>>;
pub type IntermediateBlobRepresentationStream =
	std::pin::Pin<Box<dyn Stream<Item = Result<IntermediateBlobRepresentation, DaError>> + Send>>;

/// A blob meant for the DA.
#[derive(Debug, Clone)]
pub struct DaBlob(Vec<u8>);

impl DaBlob {
	/// Creates a new [DaBlob] from a vector of bytes.
	pub fn new(data: Vec<u8>) -> Self {
		Self(data)
	}

	/// Returns a reference to the inner vector of bytes.
	pub fn as_ref(&self) -> &[u8] {
		self.0.as_slice()
	}

	/// Consumes the [DaBlob] and returns the inner vector of bytes.
	pub fn into_inner(self) -> Vec<u8> {
		self.0
	}
}

/// A height for a blob on the DA.
#[derive(Debug, Clone)]
pub struct DaHeight(u64);

impl DaHeight {
	/// Creates a new [DaHeight] from a u64.
	pub fn new(height: u64) -> Self {
		Self(height)
	}

	/// Returns the inner u64.
	pub fn as_u64(&self) -> u64 {
		self.0
	}
}

/// A certificate from consensus indicating a height.
#[derive(Debug, Clone)]
pub enum Certificate {
	/// A certificate from consensus indicating a height.
	Height(u64),
	/// A certificate that cannot be interpreted for a height.
	Nolo,
}

/// Errors thrown by [Da].
#[derive(Debug, thiserror::Error)]
pub enum DaError {
	#[error("blob submission error: {0}")]
	BlobSubmission(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("blobs at height error: {0}")]
	BlobsAtHeight(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("blobs at height fatal error: {0}")]
	BlobsAtHeightNonFatal(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("internal error: {0}")]
	Internal(String),
}

pub trait DaOperations
where
	Self: Send + Sync + 'static,
{
	/// Submits a blob to the DA.
	///
	/// A DA must allow for submission of raw [DaBlob]s and return a [IntermediateBlobRepresentation].
	fn submit_blob(
		&self,
		data: DaBlob,
	) -> impl Future<Output = Result<IntermediateBlobRepresentation, DaError>>;

	/// Gets the blobs at a given height.
	///
	/// A DA must allow for retrieval of [IntermediateBlobRepresentation]s at a given height.
	fn get_ir_blobs_at_height(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Vec<IntermediateBlobRepresentation>, DaError>> + Send + Sync + 'static;

	/// Gets the IR blobs at a given height as would be used by the stream.
	fn get_ir_blobs_at_height_for_stream(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Vec<IntermediateBlobRepresentation>, DaError>> + Send + Sync + 'static
	{
		async move {
			// get the blobs at a given height, if the error is NonFatal, return an empty vec
			match self.get_ir_blobs_at_height(height).await {
				Ok(blobs) => Ok(blobs),
				Err(DaError::BlobsAtHeightNonFatal(_)) => Ok(vec![]),
				Err(e) => Err(e),
			}
		}
	}

	/// Streams certificates from the DA.
	///
	/// A DA must allow for streaming of [Certificate]s. This is used to inform [Blob] polling.
	fn stream_certificates(&self) -> impl Future<Output = Result<CertificateStream, DaError>>;

	/// Streams [IntermediateBlobRepresentation]s from the between two heights.
	///
	/// A DA implements a standard API for streaming [IntermediateBlobRepresentation]s.
	fn stream_ir_blobs_between_heights(
		&self,
		start_height: u64,
		end_height: u64,
	) -> impl Future<Output = Result<IntermediateBlobRepresentationStream, DaError>> {
		async move {
			let stream = async_stream::try_stream! {

				for height in start_height..end_height {
					let blobs = self.get_ir_blobs_at_height_for_stream(height).await?;
					for blob in blobs {
						yield blob;
					}
				}

			};

			Ok(Box::pin(stream) as IntermediateBlobRepresentationStream)
		}
	}

	/// Streams ir blobs from a certain height.
	///
	/// A DA implements a standard API for streaming [IntermediateBlobRepresentation]s.
	fn stream_ir_blobs_from_height(
		&self,
		start_height: u64,
	) -> impl Future<Output = Result<IntermediateBlobRepresentationStream, DaError>> {
		async move {
			let stream = async_stream::try_stream! {

				// record the last height
				let mut last_height = start_height;

				// listen to the certificate stream to find the next height
				let mut certificate_stream = self.stream_certificates().await?;

				// loop through the certificate stream
				while let Some(certificate) = certificate_stream.next().await {
					match certificate {
						Ok(Certificate::Height(height)) => {
							// if the certificate height is greater than the last height, stream the blobs between the last height and the certificate height
							if height > last_height {
								let blobs = self.stream_ir_blobs_between_heights(last_height, height).await?;
								for blob in blobs {
									yield Ok(blob);
								}
								last_height = height;
							}

						}
						Ok(Certificate::Nolo) => {
							// do nothing
						}
						Err(e) => {
							yield Err(e);
						}
					}
				}

			};

			Ok(stream)
		}
	}
}
