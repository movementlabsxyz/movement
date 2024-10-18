use bridge_config::Config;
use bridge_grpc::health_client::HealthClient;
use bridge_grpc::health_server::HealthServer;
use bridge_grpc::{health_check_response::ServingStatus, HealthCheckRequest};
use bridge_service::grpc::HealthCheckService;
use std::net::SocketAddr;
use tonic::transport::Server;
use tonic::Request;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_grpc_health_check() -> Result<(), anyhow::Error> {
	// Initialize tracing for the test
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Mock config for test
	let mock_config = Config::default();

	// Initialize the gRPC health check service
	let health_service = HealthCheckService::default();
	health_service.set_service_status("", ServingStatus::Serving);

	// Set up the gRPC address based on the mock config
	let grpc_addr: SocketAddr =
		format!("{}:{}", mock_config.movement.grpc_hostname, mock_config.movement.grpc_port)
			.parse()?;
	// Spawn the gRPC server
	let grpc_server_handle = tokio::spawn(async move {
		Server::builder()
			.add_service(HealthServer::new(health_service)) // No need for Arc here
			.serve(grpc_addr)
			.await?;
		Ok::<(), tonic::transport::Error>(())
	});

	// Allow some time for the server to start
	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

	// Create the gRPC client to test the health check service
	let grpc_address =
		format!("http://{}:{}", mock_config.movement.grpc_hostname, mock_config.movement.grpc_port);

	// Connect to the gRPC server using the HealthClient
	let mut client = HealthClient::connect(grpc_address).await?;

	// Create a health check request for the overall service (empty string)
	let request = Request::new(HealthCheckRequest { service: "".to_string() });

	// Call the gRPC health check endpoint and get the response
	let response = client.check(request).await?.into_inner();

	// Assert that the health status is SERVING
	assert_eq!(response.status, ServingStatus::Serving as i32);

	// Shutdown the gRPC server
	grpc_server_handle.abort();

	Ok(())
}
