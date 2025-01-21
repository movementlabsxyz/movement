pub mod mock;

use async_stream::try_stream;
use movement_da_util::blob::ir::blob::DaBlob;
use std::future::Future;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};
use tracing::{info, warn};

pub type CertificateStream<'a> =
	Pin<Box<dyn Stream<Item = Result<Certificate, DaError>> + Send + 'a>>;
pub type DaBlobStream<'a> =
	Pin<Box<dyn Stream<Item = Result<(DaHeight, DaBlob), DaError>> + Send + 'a>>;

/// A height for a blob on the DA.
#[derive(Debug, Clone)]
pub struct DaHeight(u64);

impl DaHeight {
	pub fn new(height: u64) -> Self {
		Self(height)
	}

	pub fn as_u64(&self) -> u64 {
		self.0
	}
}

/// A certificate from consensus indicating a height.
#[derive(Debug, Clone)]
pub enum Certificate {
	Height(u64),
	Nolo,
}

/// Errors thrown by the DA.
#[derive(Debug, thiserror::Error)]
pub enum DaError {
	#[error("blob submission error: {0}")]
	BlobSubmission(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("blobs at height error: {0}")]
	BlobsAtHeight(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("non-fatal blobs at height error: {0}")]
	NonFatalBlobsAtHeight(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("certificate error: {0}")]
	Certificate(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("non-fatal certificate error: {0}")]
	NonFatalCertificate(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("internal error: {0}")]
	Internal(String),
}

/// Trait for DA operations.
pub trait DaOperations: Send + Sync {
	fn submit_blob(
		&self,
		data: DaBlob,
	) -> Pin<Box<dyn Future<Output = Result<(), DaError>> + Send + '_>>;

	fn get_da_blobs_at_height(
		&self,
		height: u64,
	) -> Pin<Box<dyn Future<Output = Result<Vec<DaBlob>, DaError>> + Send + '_>>;

	fn get_da_blobs_at_height_for_stream(
		&self,
		height: u64,
	) -> Pin<Box<dyn Future<Output = Result<Vec<DaBlob>, DaError>> + Send + '_>> {
		Box::pin(async move {
			let result = self.get_da_blobs_at_height(height).await;
			match result {
				Ok(blobs) => Ok(blobs),
				Err(e) => {
					warn!("failed to get blobs at height: {}", e);
					Ok(vec![])
				}
			}
		})
	}

	fn stream_certificates(
		&self,
	) -> Pin<Box<dyn Future<Output = Result<CertificateStream, DaError>> + Send + '_>>;

	fn stream_da_blobs_between_heights(
		&self,
		start_height: u64,
		end_height: u64,
	) -> Pin<Box<dyn Future<Output = Result<DaBlobStream, DaError>> + Send + '_>> {
		info!("streaming da blobs between heights {} and {}", start_height, end_height);
		let fut = async move {
			let stream = try_stream! {
				for height in start_height..end_height {
					info!("getting blobs at height {}", height);
					let blobs = self.get_da_blobs_at_height_for_stream(height).await?;
					for blob in blobs {
						yield (DaHeight(height), blob);
					}
				}
			};
			Ok(Box::pin(stream) as DaBlobStream)
		};
		Box::pin(fut)
	}

	fn stream_da_blobs_from_height(
		&self,
		start_height: u64,
	) -> Pin<Box<dyn Future<Output = Result<DaBlobStream, DaError>> + Send + '_>> {
		tracing::info!("TEST Da lib DaOperations stream_da_blobs_from_height start");
		let fut = async move {
			let certificate_stream = self.stream_certificates().await?;
			let stream = try_stream! {
				let mut last_height = start_height;
				let mut certificate_stream = certificate_stream;

				while let Some(certificate) = certificate_stream.next().await {

					info!("certificate: {:?}", certificate);

					match certificate {
						Ok(Certificate::Height(height)) if height > last_height => {
							tracing::info!("TEST Da lib DaOperations stream_da_blobs_from_height got new accepted blob at height:{height}");
							let blob_stream = self
								.stream_da_blobs_between_heights(last_height, height)
								.await?;
							tokio::pin!(blob_stream);

							while let Some(blob) = blob_stream.next().await {
								tracing::info!("TEST Da lib DaOperations stream_da_blobs_from_height send new accepted blob at height:{height}");
								yield blob?;
							}

							last_height = height;
						}
						// Already executed Height are use to send Heartbeat.
						Ok(Certificate::Height(height)) => {
							tracing::info!("TEST Da lib DaOperations stream_da_blobs_from_height got old height:{height}");
							//old certificate, use to send Heartbeat block.
							let blob_stream = self
								.stream_da_blobs_between_heights(height, height)
								.await?;
							tokio::pin!(blob_stream);

							while let Some(blob_res) = blob_stream.next().await {
								let (_, blob) = blob_res?;
								// Ack use heigth zero to identify heart beat block.
								// Should be changed to a type.
								let heart_blob = (DaHeight(0u64), blob);
								tracing::info!("TEST Da lib DaOperations stream_da_blobs_from_height got old height:{height} send heartbeat.");
								yield heart_blob;
							}
						}
						Ok(Certificate::Nolo) => {
							tracing::info!("TEST Da lib DaOperations stream_da_blobs_from_height got Certificate::Nolo");
							// Ignore Nolo
						}
						// Warn log non-fatal certificate errors
						Err(DaError::NonFatalCertificate(e)) => {
							warn!("non-fatal certificate error: {}", e);
						}
						// Exit on all other errors
						Err(e) => {
							yield Err(e)?;
						}
						// If height is less than last height, ignore
						// _ => {
						// 	warn!("ignoring certificate");
						// }
					}
				}
			};

			Ok(Box::pin(stream) as DaBlobStream)
		};
		Box::pin(fut)
	}
}
