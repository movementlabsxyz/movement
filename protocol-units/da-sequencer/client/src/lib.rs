use ed25519_dalek::{Verifier, VerifyingKey};
use futures::stream;
use movement_da_sequencer_proto::block_response;
use movement_da_sequencer_proto::da_sequencer_node_service_client::DaSequencerNodeServiceClient;
use movement_da_sequencer_proto::BatchWriteResponse;
use movement_da_sequencer_proto::BlockResponse;
use movement_da_sequencer_proto::BlockV1;
use movement_da_sequencer_proto::ReadAtHeightResponse;
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use movement_signer::cryptography::ed25519::PUBLIC_KEY_SIZE;
use movement_signer::cryptography::ed25519::SIGNATURE_SIZE;
use movement_signer::{
	cryptography::ed25519::{Ed25519, Signature},
	Signing,
};
use movement_signer_loader::LoadedSigner;
use std::{
	future::Future,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};
use tokio_stream::{Stream, StreamExt};
use tonic::transport::{Channel, ClientTlsConfig};
use url::Url;

/// Errors thrown by `DaSequencer`.
#[derive(Debug, thiserror::Error)]
pub enum ClientDaSequencerError {
	#[error("Fail to open block stream: {0}")]
	FailToOpenBlockStream(String),
}

pub type StreamReadBlockFromHeight =
	std::pin::Pin<Box<dyn Stream<Item = Result<BlockV1, ClientDaSequencerError>> + Send + 'static>>;

pub trait DaSequencerClient: Clone + Send {
	/// Stream reads from a given height.
	fn stream_read_from_height(
		&mut self,
		request: StreamReadFromHeightRequest,
	) -> impl Future<
		Output = Result<(StreamReadBlockFromHeight, UnboundedReceiver<()>), ClientDaSequencerError>,
	> + Send;

	/// Writes a batch of transactions to the Da Sequencer node
	fn batch_write(
		&mut self,
		request: movement_da_sequencer_proto::BatchWriteRequest,
	) -> impl Future<Output = Result<BatchWriteResponse, tonic::Status>> + Send;
	fn send_state(
		&mut self,
		signer: &LoadedSigner<Ed25519>,
		state: movement_da_sequencer_proto::MainNodeState,
	) -> impl Future<Output = Result<BatchWriteResponse, tonic::Status>> + Send;

	fn read_at_height(
		&mut self,
		height: u64,
	) -> impl Future<Output = Result<ReadAtHeightResponse, tonic::Status>> + Send;
}

/// Grpc implementation of the DA Sequencer client
#[derive(Debug, Clone)]
pub struct GrpcDaSequencerClient {
	client: DaSequencerNodeServiceClient<tonic::transport::Channel>,
	pub stream_heartbeat_interval_sec: u64,
}

impl GrpcDaSequencerClient {
	/// Creates an http2 connection to the Da Sequencer node service.
	pub async fn try_connect(
		connection_url: &Url,
		stream_heartbeat_interval_sec: u64,
	) -> Result<Self, anyhow::Error> {
		for _ in 0..5 {
			match GrpcDaSequencerClient::connect(connection_url.clone()).await {
				Ok(client) => {
					return Ok(GrpcDaSequencerClient { client, stream_heartbeat_interval_sec });
				}
				Err(err) => {
					tracing::warn!(
						"DA sequencer Http2 connection url:{connection_url} failed: {}. Retrying in 10s...",
						err
					);
					let _ = tokio::time::sleep(Duration::from_secs(10)).await;
				}
			}
		}

		Err(anyhow::anyhow!(
			"Error DA Sequencer Http2 connection failed more than 5 times. Aborting."
		))
	}

	/// Connects to a da sequencer node service using the given connection string.
	async fn connect(
		connection_url: Url,
	) -> Result<DaSequencerNodeServiceClient<tonic::transport::Channel>, anyhow::Error> {
		tracing::info!("Grpc client connect using :{connection_url}");
		let endpoint = Channel::from_shared(connection_url.as_str().to_string())?;

		// Dynamically configure TLS based on the scheme (http or https)
		let endpoint = if connection_url.scheme() == ("https") {
			endpoint
				.tls_config(ClientTlsConfig::new().with_enabled_roots())?
				.http2_keep_alive_interval(Duration::from_secs(10))
		} else {
			endpoint
		};

		let channel = endpoint.connect().await?;
		let client = DaSequencerNodeServiceClient::new(channel);

		Ok(client)
	}
}

impl DaSequencerClient for GrpcDaSequencerClient {
	/// Stream reads from a given hestreamight.
	async fn stream_read_from_height(
		&mut self,
		request: StreamReadFromHeightRequest,
	) -> Result<(StreamReadBlockFromHeight, UnboundedReceiver<()>), ClientDaSequencerError> {
		let start_height = if request.height == 0 { 1 } else { request.height };

		let response = self
			.client
			.stream_read_from_height(request)
			.await
			.map_err(|err| ClientDaSequencerError::FailToOpenBlockStream(err.to_string()))?;

		let mut stream = response.into_inner();
		let last_heartbeat_time = Arc::new(Mutex::new(Instant::now()));

		let (alert_tx, alert_rx) = unbounded_channel();
		let last_heartbeat_time = Arc::clone(&last_heartbeat_time);
		let heartbeat_interval = Duration::from_secs(self.stream_heartbeat_interval_sec);
		let missed_heartbeat_threshold = heartbeat_interval * 2;

		// Start missing heartbeat loop.
		tokio::spawn({
			let last_heartbeat_time = Arc::clone(&last_heartbeat_time);
			async move {
				loop {
					tokio::time::sleep(heartbeat_interval).await;
					let elapsed = last_heartbeat_time.lock().await.elapsed();
					if elapsed > missed_heartbeat_threshold {
						let _ = alert_tx.send(());
						break;
					}
				}
			}
		});

		let output = async_stream::try_stream! {
			// Block da height is monotonic.
			let mut expected_height = start_height;
			loop {
				match stream.next().await {
					Some(Ok(block_response)) => {
						match block_response.response {
							Some(response) => match response.block_type {
								Some(block_response::BlockType::Heartbeat(_)) => {
									tracing::info!("Received heartbeat");
									*last_heartbeat_time.lock().await = Instant::now();
								}
								Some(block_response::BlockType::BlockV1(block)) => {
									// Detect non consecutive height.
									if block.height != expected_height {
										tracing::error!("Not an expected block height from DA: expected:{expected_height} received:{}", block.height);
										// only break because we don't report error in the stream.
										// The client re connection will detect end of heartbeat and reconnect.
										break;
									} else {
										expected_height +=1;
										*last_heartbeat_time.lock().await = Instant::now();
										yield block;

									}
								}
								None =>  {
									tracing::error!("Da sequencer client stream return a none block. Da height not available, break.");
									break
								},
							},
							None => {
								tracing::error!("Da sequencer client stream return non. Stream closed.");
								break
							}
						}
					}
					Some(Err(err)) => {
						tracing::error!("Da sequencer client connection return an error:{err}");
						break;
					}
					None => {
						tracing::error!("Da sequencer client connection return None.");
						break;
					}
				}
			}
		};

		Ok((Box::pin(output) as StreamReadBlockFromHeight, alert_rx))
	}

	/// Writes a batch of transactions to the Da Sequencer node
	async fn batch_write(
		&mut self,
		request: movement_da_sequencer_proto::BatchWriteRequest,
	) -> Result<BatchWriteResponse, tonic::Status> {
		let response = self.client.batch_write(request).await?;
		Ok(response.into_inner())
	}

	async fn send_state(
		&mut self,
		signer: &LoadedSigner<Ed25519>,
		state: movement_da_sequencer_proto::MainNodeState,
	) -> Result<BatchWriteResponse, tonic::Status> {
		let serialized = serialize_node_state(&state);
		let signature = signer.sign(&serialized).await.map_err(|err| {
			tonic::Status::new(tonic::Code::Unauthenticated, format!("State signgin failed: {err}"))
		})?;

		let request = movement_da_sequencer_proto::MainNodeStateRequest {
			state: Some(state),
			signature: signature.as_bytes().to_vec(),
		};
		let response = self.client.send_state(request).await?;
		Ok(response.into_inner())
	}

	async fn read_at_height(&mut self, height: u64) -> Result<ReadAtHeightResponse, tonic::Status> {
		let request = movement_da_sequencer_proto::ReadAtHeightRequest { height };
		let response = self.client.read_at_height(request).await?;
		Ok(response.into_inner())
	}
}

pub fn serialize_node_state(state: &movement_da_sequencer_proto::MainNodeState) -> Vec<u8> {
	let mut serialized: Vec<u8> = Vec::with_capacity(8 + 8 + 8);
	serialized.extend_from_slice(&state.block_height.to_le_bytes());
	serialized.extend_from_slice(&state.ledger_timestamp.to_le_bytes());
	serialized.extend_from_slice(&state.ledger_version.to_le_bytes());
	serialized
}

/// Signs and encodes a batch for submission to the DA Sequencer.
pub async fn sign_and_encode_batch(
	batch_data: Vec<u8>,
	signer: &LoadedSigner<Ed25519>,
) -> Result<Vec<u8>, anyhow::Error> {
	let signature = signer.sign(&batch_data).await?;
	let verifying_key =
		ed25519_dalek::VerifyingKey::from_bytes(&signer.public_key().await?.to_bytes())?;
	Ok(serialize_full_node_batch(verifying_key, signature, batch_data))
}

/// Serializes a full node batch with verifying key and signature prepended.
pub fn serialize_full_node_batch(
	verifying_key: VerifyingKey,
	signature: Signature,
	mut data: Vec<u8>,
) -> Vec<u8> {
	let mut serialized: Vec<u8> = Vec::with_capacity(PUBLIC_KEY_SIZE + SIGNATURE_SIZE + data.len());
	serialized.extend_from_slice(&verifying_key.to_bytes());
	serialized.extend_from_slice(&signature.as_bytes());
	serialized.append(&mut data);
	serialized
}

/// Deserializes a full node batch into verifying key, signature, and data.
pub fn deserialize_full_node_batch(
	data: Vec<u8>,
) -> Result<(VerifyingKey, ed25519_dalek::Signature, Vec<u8>), anyhow::Error> {
	if data.len() < PUBLIC_KEY_SIZE + SIGNATURE_SIZE {
		return Err(anyhow::anyhow!("Data len to small to deserialize."));
	}
	let (pubkey_deserialized, rest) = data.split_at(PUBLIC_KEY_SIZE);
	let (sign_deserialized, vec_deserialized) = rest.split_at(SIGNATURE_SIZE);

	// Convert the slices back into arrays
	let pub_key_bytes: [u8; PUBLIC_KEY_SIZE] = pubkey_deserialized.try_into()?;
	let signature_bytes: [u8; SIGNATURE_SIZE] = sign_deserialized.try_into()?;

	let verifying_key = VerifyingKey::try_from(pub_key_bytes.as_slice())?;
	let signature = ed25519_dalek::Signature::try_from(signature_bytes.as_slice())?;

	let data: Vec<u8> = vec_deserialized.to_vec();
	Ok((verifying_key, signature, data))
}

/// Verifies a batch signature using the given verifying key.
pub fn verify_batch_signature(
	batch_data: &[u8],
	signature: &ed25519_dalek::Signature,
	verifying_key: &VerifyingKey,
) -> Result<(), anyhow::Error> {
	Ok(verifying_key.verify(batch_data, signature)?)
}

/// A DaSequencerClient implementation that no nothing. Can be used to mock the DA.
#[derive(Clone)]
pub struct EmptyDaSequencerClient;

impl DaSequencerClient for EmptyDaSequencerClient {
	/// Stream reads from a given height.
	async fn stream_read_from_height(
		&mut self,
		_request: movement_da_sequencer_proto::StreamReadFromHeightRequest,
	) -> Result<(StreamReadBlockFromHeight, UnboundedReceiver<()>), ClientDaSequencerError> {
		let never_ending_stream = stream::pending::<Result<BlockV1, ClientDaSequencerError>>();
		let (_alert_tx, alert_rx) = unbounded_channel();

		Ok((Box::pin(never_ending_stream), alert_rx))
	}

	/// Writes a batch of transactions to the Da Sequencer node
	async fn batch_write(
		&mut self,
		_request: movement_da_sequencer_proto::BatchWriteRequest,
	) -> Result<BatchWriteResponse, tonic::Status> {
		Ok(BatchWriteResponse { answer: true })
	}
	async fn send_state(
		&mut self,
		_signer: &LoadedSigner<Ed25519>,
		_state: movement_da_sequencer_proto::MainNodeState,
	) -> Result<BatchWriteResponse, tonic::Status> {
		Ok(BatchWriteResponse { answer: true })
	}

	async fn read_at_height(
		&mut self,
		_height: u64,
	) -> Result<ReadAtHeightResponse, tonic::Status> {
		Ok(ReadAtHeightResponse { response: Some(BlockResponse { block_type: None }) })
	}
}
