use anyhow::Result;
use ed25519_dalek::{Signature, Signer, SigningKey};
use futures_core::Stream;
use futures_util::stream::unfold;
use movement_da_sequencer_proto::da_sequencer_node_service_client::DaSequencerNodeServiceClient;
use movement_da_sequencer_proto::{
	BatchWriteRequest, BatchWriteResponse, StreamReadFromHeightRequest,
	StreamReadFromHeightResponse,
};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::{Status, Streaming};

#[derive(Debug, Clone)]
pub struct DaSequencerClient {
	client: DaSequencerNodeServiceClient<Channel>,
	connection_string: String,
	last_received_height: Arc<Mutex<Option<u64>>>,
}

impl DaSequencerClient {
	pub async fn try_connect(connection_string: &str) -> Result<Self> {
		for _ in 0..5 {
			match Self::connect(connection_string).await {
				Ok(client) => {
					return Ok(Self {
						client,
						connection_string: connection_string.to_string(),
						last_received_height: Arc::new(Mutex::new(None)),
					});
				}
				Err(err) => {
					tracing::warn!(
						"DA sequencer HTTP/2 connection failed: {}. Retrying in 2s...",
						err
					);
					tokio::time::sleep(Duration::from_secs(2)).await;
				}
			}
		}

		Err(anyhow::anyhow!("Connection failed more than 5 times"))
	}

	async fn connect(connection_string: &str) -> Result<DaSequencerNodeServiceClient<Channel>> {
		tracing::info!("Grpc client connect using: {}", connection_string);
		let endpoint = Channel::from_shared(connection_string.to_string())?;

		let endpoint = if connection_string.starts_with("https://") {
			endpoint
				.tls_config(ClientTlsConfig::new().with_enabled_roots())?
				.http2_keep_alive_interval(Duration::from_secs(10))
		} else {
			endpoint
		};

		let channel = endpoint.connect().await?;
		Ok(DaSequencerNodeServiceClient::new(channel))
	}

	async fn reconnect(&mut self) -> Result<()> {
		tracing::info!("Reconnecting to {}", self.connection_string);
		let client = Self::connect(&self.connection_string).await?;
		self.client = client;
		Ok(())
	}

	pub async fn stream_read_from_height(
		&mut self,
		start_request: StreamReadFromHeightRequest,
	) -> Result<
		Pin<Box<dyn Stream<Item = Result<StreamReadFromHeightResponse, Status>> + Send>>,
		Status,
	> {
		let height = {
			let last = self.last_received_height.lock().unwrap();
			if let Some(last_h) = *last {
				tracing::info!("Resuming stream from height: {}", last_h + 1);
				last_h + 1
			} else {
				tracing::info!("Starting stream from requested height: {}", start_request.height);
				start_request.height
			}
		};

		match self
			.client
			.stream_read_from_height(StreamReadFromHeightRequest { height })
			.await
		{
			Ok(response) => Ok(Self::wrap_stream_with_height_tracking(
				response.into_inner(),
				Arc::clone(&self.last_received_height),
			)),
			Err(e) => {
				tracing::warn!("stream_read_from_height failed, trying reconnect: {e}");
				self.reconnect()
					.await
					.map_err(|e| Status::unavailable(format!("Reconnect failed: {e}")))?;

				let response = self
					.client
					.stream_read_from_height(StreamReadFromHeightRequest { height })
					.await?;

				Ok(Self::wrap_stream_with_height_tracking(
					response.into_inner(),
					Arc::clone(&self.last_received_height),
				))
			}
		}
	}

	fn wrap_stream_with_height_tracking(
		stream: Streaming<StreamReadFromHeightResponse>,
		last_received_height: Arc<Mutex<Option<u64>>>,
	) -> Pin<Box<dyn Stream<Item = Result<StreamReadFromHeightResponse, Status>> + Send>> {
		let wrapped = unfold((stream, last_received_height), |(mut s, tracker)| async move {
			match s.message().await {
				Ok(Some(msg)) => {
					if let Some(ref blob) = msg.response {
						if let Some(height) = blob.blob_type.as_ref().and_then(|b| match b {
							movement_da_sequencer_proto::blob_response::BlobType::Blockv1(
								inner,
							) => Some(inner.height),
							_ => None,
						}) {
							*tracker.lock().unwrap() = Some(height);
						}
					}
					Some((Ok(msg), (s, tracker)))
				}
				Ok(None) => None,
				Err(e) => Some((Err(e), (s, tracker))),
			}
		});

		Box::pin(wrapped)
	}

	pub async fn batch_write(
		&mut self,
		request: BatchWriteRequest,
	) -> Result<BatchWriteResponse, Status> {
		match self.client.batch_write(request.clone()).await {
			Ok(response) => Ok(response.into_inner()),
			Err(_) => {
				self.reconnect()
					.await
					.map_err(|e| Status::unavailable(format!("Reconnect failed: {}", e)))?;

				let response = self.client.batch_write(request).await?;
				Ok(response.into_inner())
			}
		}
	}
}

pub fn sign_batch(batch_data: &[u8], signing_key: &SigningKey) -> Signature {
	signing_key.sign(batch_data)
}
