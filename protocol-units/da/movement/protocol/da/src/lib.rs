use movement_da_light_node_proto::Blob;
use movement_da_util::blob::ir::blob::IntermediateBlobRepresentation;
use std::error;
use std::future::Future;

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

pub trait DaOperations {
	/// Submits a blob to the DA.
	///
	/// A DA must allow for submission of raw [DaBlob]s and return a [Blob].
	fn submit_blob(&self, data: DaBlob) -> impl Future<Output = Result<Blob, DaError>>;

	/// Gets the blobs at a given height.
	///
	/// A DA must allow for retrieval of [IntermediateBlobRepresentation]s at a given height.
	fn get_ir_blobs_at_height(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Vec<IntermediateBlobRepresentation>, DaError>>;

	/// Gets the IR blobs at a given height as would be used by the stream.
	fn get_ir_blobs_at_height_for_stream(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Vec<IntermediateBlobRepresentation>, DaError>> {
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
	fn stream_certificates(&self) -> impl futures::Stream<Item = Result<Certificate, DaError>>;
}
