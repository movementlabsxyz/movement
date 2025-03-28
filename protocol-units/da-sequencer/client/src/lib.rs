use ed25519_dalek::{SigningKey, Signature};
use ed25519_dalek::Signer;
use movement_da_sequencer_proto::da_sequencer_node_service_client::DaSequencerNodeServiceClient;
use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig};

/// An enum wrapping MovementDaLightNodeClients over complex types.
///
/// The usage of hype by tonic and related libraries makes it very difficult to maintain generic types for the clients. This enum simplifies client construction and usage.
#[derive(Debug, Clone)]
pub struct DaSequencerClient {
	client: DaSequencerNodeServiceClient<tonic::transport::Channel>,
}

impl DaSequencerClient {
	/// Creates an http2 connection to the Da Sequencer node service.
	pub async fn try_connect(&self, connection_string: &str) -> Result<Self, anyhow::Error> {
		for _ in 0..5 {
			match DaSequencerClient::connect(connection_string).await {
				Ok(client) => return Ok(DaSequencerClient { client }),
				Err(err) => {
					tracing::warn!(
						"DA sequencer Http2 connection failed: {}. Retrying in 10s...",
						err
					);
					std::thread::sleep(std::time::Duration::from_secs(10));
				}
			}
		}
		return Err(anyhow::anyhow!(
			"Error DA Sequencer Http2 connection failed more than 5 time aborting.",
		));
	}

	/// Stream reads from a given height.
	pub async fn stream_read_from_height(
		&mut self,
		request: movement_da_sequencer_proto::StreamReadFromHeightRequest,
	) -> Result<
		tonic::Streaming<movement_da_sequencer_proto::StreamReadFromHeightResponse>,
		tonic::Status,
	> {
		let response = self.client.stream_read_from_height(request).await?;
		Ok(response.into_inner())
	}

	/// Writes a batch of transactions to the light node
	pub async fn batch_write(
		&mut self,
		request: movement_da_sequencer_proto::BatchWriteRequest,
	) -> Result<movement_da_sequencer_proto::BatchWriteResponse, tonic::Status> {
		let response = self.client.batch_write(request).await?;
		Ok(response.into_inner())
	}
	
	/// Connects to a da sequencer node service using the given connection string.
	async fn connect(
		connection_string: &str,
	) -> Result<DaSequencerNodeServiceClient<tonic::transport::Channel>, anyhow::Error> {
		let endpoint = Channel::from_shared(connection_string.to_string())?;

		// Dynamically configure TLS based on the scheme (http or https)
		let endpoint = if connection_string.starts_with("https://") {
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

pub fn sign_batch(batch_data: &[u8], signing_key: &SigningKey) -> Signature {
        signing_key.sign(batch_data)
}
