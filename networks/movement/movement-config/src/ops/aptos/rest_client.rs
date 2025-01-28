use crate::Config;
use aptos_sdk::rest_client::Client;
use std::future::Future;

/// Errors thrown when attempting to use the config for an Aptos rest client.
#[derive(Debug, thiserror::Error)]
pub enum RestClientError {
	#[error("building client failed: {0}")]
	BuildingClient(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// A trait for [RestClient] operations.
///
/// This is useful for managing imports and adding sub implementations.
pub trait RestClient {
	fn get_rest_client(&self) -> impl Future<Output = Result<Client, RestClientError>>;
}

impl RestClient for Config {
	async fn get_rest_client(&self) -> Result<Client, RestClientError> {
		// get the relevant fields from the config
		let protocol = "http";
		let hostname = self
			.execution_config
			.maptos_config
			.client
			.maptos_rest_connection_hostname
			.clone();
		let port = self.execution_config.maptos_config.client.maptos_rest_connection_port;

		// build the connection string
		let connection_string = format!("{}://{}:{}", protocol, hostname, port);

		// build the client
		let client = Client::new(connection_string.parse().map_err(|e| {
			RestClientError::BuildingClient(
				format!("failed to parse connection string: {}", e).into(),
			)
		})?);

		Ok(client)
	}
}
