use movement_da_util::ir_blob::IntermediateBlobRepresentation;
use movement_da_light_node_proto::Blob;
use std::error;
use std::future::Future;

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
	#[error("blobs at height fatal error: {0}")]
	BlobsAtHeightFatal(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("blobs at height error: {0}")]
	BlobsAtHeight(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("internal error: {0}")]
	Internal(String),
}

pub trait DaOperations {
	/// Submits a blob to the DA.
	///
	/// A DA must allow for submission of raw blobs.
	fn submit_blob(&self, data: Vec<u8>) -> impl Future<Output = Result<Blob, DaError>>;

	/// Gets the blobs at a given height.
	///
	/// A DA must allow for retrieval of [IntermediateBlobRepresentation]s at a given height.
	fn get_ir_blobs_at_height(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Vec<IntermediateBlobRepresentation>, DaError>>;

	/// Streams certificates from the DA.
	///
	/// A DA must allow for streaming of [Certificate]s. This is used to inform [Blob] polling.
	fn stream_certificates(&self) -> impl futures::Stream<Item = Result<Certificate, DaError>>;
}
