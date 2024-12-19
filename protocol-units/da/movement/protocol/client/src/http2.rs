use movement_da_light_node_proto::light_node_service_client::LightNodeServiceClient;

#[derive(Debug, Clone)]
pub struct Http2 {
	client: LightNodeServiceClient<tonic::transport::Channel>,
}

impl Http2 {
	/// Connects to a light node service using the given connection string.
	pub async fn connect(connection_string: &str) -> Result<Self, anyhow::Error> {
		let client = LightNodeServiceClient::connect(connection_string.to_string()).await?;
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
