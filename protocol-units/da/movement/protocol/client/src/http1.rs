use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use movement_da_light_node_proto::light_node_service_client::LightNodeServiceClient;
use tonic_web::GrpcWebClientLayer;
use tonic_web::{GrpcWebCall, GrpcWebClientService};

#[derive(Debug, Clone)]
pub struct Http1 {
	client: LightNodeServiceClient<
		GrpcWebClientService<
			Client<
				HttpConnector,
				GrpcWebCall<
					http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, tonic::Status>,
				>,
			>,
		>,
	>,
}

impl Http1 {
	/// Tries to connect to a light node service using the given connection string.
	pub fn try_new(connection_string: &str) -> Result<Self, anyhow::Error> {
		let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build_http();

		let svc = tower::ServiceBuilder::new().layer(GrpcWebClientLayer::new()).service(client);

		let client = LightNodeServiceClient::with_origin(svc, connection_string.try_into()?);

		Ok(Http1 { client })
	}

	/// Returns a reference to the client.
	pub fn client(
		&self,
	) -> &LightNodeServiceClient<
		GrpcWebClientService<
			Client<
				HttpConnector,
				GrpcWebCall<
					http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, tonic::Status>,
				>,
			>,
		>,
	> {
		&self.client
	}

	/// Returns a mutable reference to the client.
	pub fn client_mut(
		&mut self,
	) -> &mut LightNodeServiceClient<
		GrpcWebClientService<
			Client<
				HttpConnector,
				GrpcWebCall<
					http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, tonic::Status>,
				>,
			>,
		>,
	> {
		&mut self.client
	}
}
