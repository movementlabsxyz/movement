pub mod http1;
pub mod http2;

/// An enum wrapping MovementDaLightNodeClients over complex types.
///
/// The usage of hype by tonic and related libraries makes it very difficult to maintain generic types for the clients. This enum simplifies client construction and usage.
#[derive(Debug, Clone)]
pub enum MovementDaLightNodeClient {
	/// An http1 client.
	Http1(http1::Http1),
	/// An http2 client.
	Http2(http2::Http2),
}

impl MovementDaLightNodeClient {
	/// Creates an http1 connection to the light node service.
	pub fn try_http1(connection_string: &str) -> Result<Self, anyhow::Error> {
		for _ in 0..5 {
			match http1::Http1::try_new(connection_string) {
				Ok(result) => return Ok(Self::Http1(result)),
				Err(err) => {
					tracing::warn!("DA Http1 connection failed: {}. Retrying in 5s...", err);
					std::thread::sleep(std::time::Duration::from_secs(5));
				}
			}
		}
		return Err(
			anyhow::anyhow!("Error DA Http1 connection failed more than 5 time aborting.",),
		);
	}

	/// Creates an http2 connection to the light node service.
	pub async fn try_http2(connection_string: &str) -> Result<Self, anyhow::Error> {
		for _ in 0..5 {
			match http2::Http2::connect(connection_string).await {
				Ok(result) => return Ok(Self::Http2(result)),
				Err(err) => {
					tracing::warn!("DA Http2 connection failed: {}. Retrying in 5s...", err);
					std::thread::sleep(std::time::Duration::from_secs(5));
				}
			}
		}
		return Err(
			anyhow::anyhow!("Error DA Http2 connection failed more than 5 time aborting.",),
		);
	}

	/// Stream reads from a given height.
	pub async fn stream_read_from_height(
		&mut self,
		request: movement_da_light_node_proto::StreamReadFromHeightRequest,
	) -> Result<
		tonic::Streaming<movement_da_light_node_proto::StreamReadFromHeightResponse>,
		tonic::Status,
	> {
		match self {
			Self::Http1(client) => {
				let response = client.client_mut().stream_read_from_height(request).await?;
				Ok(response.into_inner())
			}
			Self::Http2(client) => {
				let response = client.client_mut().stream_read_from_height(request).await?;
				Ok(response.into_inner())
			}
		}
	}

	/// Writes a batch of transactions to the light node
	pub async fn batch_write(
		&mut self,
		request: movement_da_light_node_proto::BatchWriteRequest,
	) -> Result<movement_da_light_node_proto::BatchWriteResponse, tonic::Status> {
		match self {
			Self::Http1(client) => {
				let response = client.client_mut().batch_write(request).await?;
				Ok(response.into_inner())
			}
			Self::Http2(client) => {
				let response = client.client_mut().batch_write(request).await?;
				Ok(response.into_inner())
			}
		}
	}
}
