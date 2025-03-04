use movement_da_light_node_proto::light_node_service_client::LightNodeServiceClient;
use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig};

#[derive(Debug, Clone)]
pub struct Http2 {
	client: LightNodeServiceClient<tonic::transport::Channel>,
}

impl Http2 {
	/// Connects to a light node service using the given connection string.
	pub async fn connect(connection_string: &str) -> Result<Self, anyhow::Error> {
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
		let client = LightNodeServiceClient::new(channel);

		Ok(Http2 { client })
	}

	/// Returns a reference to the client.
	pub fn client(&self) -> &LightNodeServiceClient<tonic::transport::Channel> {
		&self.client
	}

	/// Returns a mutable reference to the client.
	pub fn client_mut(&mut self) -> &mut LightNodeServiceClient<tonic::transport::Channel> {
		&mut self.client
	}
}
