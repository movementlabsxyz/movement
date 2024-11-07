use bridge_config::Config;
use bridge_grpc::{
	health_check_response::ServingStatus, health_client::HealthClient, health_server::HealthServer,
	HealthCheckRequest,
};
use bridge_service::grpc::HealthCheckService;
use std::net::SocketAddr;
use tonic::{transport::Server, Request};
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
	let grpc_addr: SocketAddr = format!(
		"{}:{}",
		mock_config.movement.grpc_listener_hostname, mock_config.movement.grpc_port
	)
	.parse()?;
	// Spawn the gRPC server
	let grpc_server_handle = tokio::spawn(async move {
		Server::builder()
			.add_service(HealthServer::new(health_service)) // No need for Arc here
			.serve(grpc_addr)
			.await?;
		Ok::<(), tonic::transport::Error>(())
	});

	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

	let grpc_address = format!(
		"http://{}:{}",
		mock_config.movement.grpc_listener_hostname, mock_config.movement.grpc_port
	);

	let mut client = HealthClient::connect(grpc_address).await?;
	let request = Request::new(HealthCheckRequest { service: "".to_string() });
	let response = client.check(request).await?.into_inner();

	assert_eq!(response.status, ServingStatus::Serving as i32);

	grpc_server_handle.abort();
	Ok(())
}
