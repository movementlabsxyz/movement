use crate::{Certificate, CertificateStream, DaError, DaOperations};
use movement_da_util::blob::ir::blob::DaBlob;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// A mock DA implementation, useful for testing.
pub struct Mock {
	// A queue for certificates.
	certificate_queue: Arc<Mutex<VecDeque<Result<Certificate, DaError>>>>,

	// Map for mocking results of `get_da_blobs_at_height`.
	height_results: Arc<Mutex<HashMap<u64, Result<Vec<DaBlob>, DaError>>>>,

	// Collection to store submitted blobs.
	submitted_blobs: Arc<Mutex<Vec<DaBlob>>>,
}

impl Mock {
	/// Creates a new `Mock` instance.
	pub fn new() -> Self {
		Self {
			certificate_queue: Arc::new(Mutex::new(VecDeque::new())),
			height_results: Arc::new(Mutex::new(HashMap::new())),
			submitted_blobs: Arc::new(Mutex::new(Vec::new())),
		}
	}

	/// Adds a certificate to the queue.
	pub fn add_certificate(
		&self,
		certificate: Result<Certificate, DaError>,
	) -> Result<(), DaError> {
		let mut queue = self.certificate_queue.lock().map_err(|_| {
			DaError::Internal("Failed to acquire lock for certificate queue".to_string())
		})?;
		queue.push_back(certificate);
		Ok(())
	}

	/// Sets the result for a specific height.
	pub fn set_height_result(
		&self,
		height: u64,
		result: Result<Vec<DaBlob>, DaError>,
	) -> Result<(), DaError> {
		let mut height_results = self.height_results.lock().map_err(|_| {
			DaError::Internal("Failed to acquire lock for height results".to_string())
		})?;
		height_results.insert(height, result);
		Ok(())
	}

	/// Gets all submitted blobs.
	pub fn get_submitted_blobs(&self) -> Result<Vec<DaBlob>, DaError> {
		self.submitted_blobs.lock().map(|blobs| blobs.clone()).map_err(|_| {
			DaError::Internal("Failed to acquire lock for submitted blobs".to_string())
		})
	}
}

impl DaOperations for Mock {
	fn submit_blob(
		&self,
		data: DaBlob,
	) -> Pin<Box<dyn Future<Output = Result<(), DaError>> + Send + '_>> {
		let submitted_blobs = self.submitted_blobs.clone();
		Box::pin(async move {
			submitted_blobs
				.lock()
				.map_err(|_| {
					DaError::Internal("Failed to acquire lock for submitted blobs".to_string())
				})?
				.push(data);
			Ok(())
		})
	}

	fn get_da_blobs_at_height(
		&self,
		height: u64,
	) -> Pin<Box<dyn Future<Output = Result<Vec<DaBlob>, DaError>> + Send + '_>> {
		let height_results = self.height_results.clone();
		Box::pin(async move {
			height_results
				.lock()
				.map_err(|_| {
					DaError::Internal("Failed to acquire lock for height results".to_string())
				})?
				.remove(&height)
				.ok_or_else(|| DaError::Internal(format!("No result set for height {}", height)))?
		})
	}

	fn stream_certificates(
		&self,
	) -> Pin<Box<dyn Future<Output = Result<CertificateStream, DaError>> + Send + '_>> {
		let certificate_queue = self.certificate_queue.clone();

		Box::pin(async move {
			// Create an mpsc channel for streaming certificates.
			let (sender, receiver) = mpsc::channel(10);

			// Move certificates from the queue into the channel in a background task.
			let queue_worker = async move {
				loop {
					// Lock the queue and pop the next certificate.
					let certificate = {
						let mut queue = certificate_queue.lock().unwrap();
						queue.pop_front()
					};

					match certificate {
						Some(cert) => {
							if sender.send(cert).await.is_err() {
								break; // Stop if the receiver has been dropped.
							}
						}
						None => break, // Exit the loop when the queue is empty.
					}
				}
			};

			tokio::spawn(queue_worker);

			// Wrap the receiver in a `ReceiverStream` and return it.
			let stream = ReceiverStream::new(receiver);
			Ok(Box::pin(stream) as CertificateStream)
		})
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use tokio_stream::StreamExt;

	#[tokio::test]
	async fn test_stream_stays_open_with_non_fatal_certificate() -> Result<(), anyhow::Error> {
		// Create a mock DA instance.
		let mock = Mock::new();

		// Add a mix of valid certificates and a non-fatal error to the queue.
		mock.add_certificate(Ok(Certificate::Height(1)))?;
		mock.add_certificate(Err(DaError::NonFatalCertificate(
			"non-fatal error".to_string().into(),
		)))?;
		mock.add_certificate(Ok(Certificate::Height(2)))?;

		// Get the stream of certificates.
		let certificate_stream = mock.stream_certificates().await?;
		tokio::pin!(certificate_stream);

		let mut results = Vec::new();

		// Process the stream.
		while let Some(cert) = certificate_stream.next().await {
			match cert {
				Ok(Certificate::Height(height)) => results.push(Ok(height)),
				Err(e) => results.push(Err(e.to_string())),
				_ => {}
			}
		}

		// Validate the results.
		assert_eq!(
			results,
			vec![
				Ok(1),                                                           // First certificate
				Err("non-fatal certificate error: non-fatal error".to_string()), // Non-fatal error
				Ok(2),                                                           // Second certificate
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn test_stream_closes_with_fatal() -> Result<(), anyhow::Error> {
		// Create a mock DA instance.
		let mock = Mock::new();

		// Add a mix of valid certificates and a fatal error to the queue.
		mock.add_certificate(Ok(Certificate::Height(1)))?;
		mock.add_certificate(Err(DaError::Internal("fatal error".to_string())))?;
		mock.add_certificate(Ok(Certificate::Height(2)))?;

		// Get the stream of certificates.
		let certificate_stream = mock.stream_certificates().await?;
		tokio::pin!(certificate_stream);

		let mut results = Vec::new();

		// Process the stream.
		while let Some(cert) = certificate_stream.next().await {
			match cert {
				Ok(Certificate::Height(height)) => results.push(Ok(height)),
				Err(e) => results.push(Err(e.to_string())),
				_ => {}
			}
		}

		// Validate the results.
		assert_eq!(
			results,
			vec![
				Ok(1),                                          // First certificate
				Err("internal error: fatal error".to_string()), // Fatal error
			]
		);

		Ok(())
	}
}
