use bridge_config::Config;
use bridge_service::grpc_health_check::Health;
use bridge_service::grpc_health_check::HealthCheckService;
use movementlabs_protocol_units_bridge_v1beta1::health_client::HealthClient;
use movementlabs_protocol_units_bridge_v1beta1::{
	health_check_response::ServingStatus, HealthCheckRequest,
};
use std::net::SocketAddr;
use tokio::sync::Arc;
use tonic::transport::Channel;
use tonic::transport::Server;
use tonic::Request;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_grpc_health_check() -> Result<(), Box<dyn std::error::Error>> {
	// Initialize tracing for the test
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Create mock config
	let mock_config = Config::default();

	// Initialize the gRPC health check service
	let health_service = Arc::new(HealthCheckService::default());
	health_service.set_service_status("", ServingStatus::SERVING);

	// Start the gRPC server
	let grpc_addr: SocketAddr = "[::1]:50051".parse()?;
	let grpc_server_handle = tokio::spawn(async move {
		Server::builder()
			.add_service(HealthServer::new(health_service.clone()))
			.serve(grpc_addr)
			.await?;
		Ok::<(), tonic::transport::Error>(())
	});

	// Give the server a moment to start
	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

	// Create a gRPC client to test the health check endpoint
	let mut client = HealthClient::connect("http://[::1]:50051").await?;

	// Create a health check request for the overall service (empty string for all services)
	let request = Request::new(HealthCheckRequest { service: "".to_string() });

	// Call the gRPC health check endpoint and get the response
	let response = client.check(request).await?.into_inner();

	// Assert that the health status is SERVING
	assert_eq!(response.status, ServingStatus::SERVING as i32);

	// Shutdown the gRPC server
	grpc_server_handle.abort();

	Ok(())
}
