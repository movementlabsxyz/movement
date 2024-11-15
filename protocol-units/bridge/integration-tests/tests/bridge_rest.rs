use bridge_config::Config;
use bridge_service::rest::BridgeRest;
use poem::test::TestClient;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_rest_service_health_endpoint() -> Result<(), anyhow::Error> {
	// Initialize tracing for the test
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Create mock config
	let mock_config = Config::default();

	// Create the REST service, unwrapping the result
	let (health_tx, mut health_rx) = tokio::sync::mpsc::channel(10);
	let rest_service = Arc::new(BridgeRest::new(&mock_config.movement, health_tx)?);

	let rest_service_for_task = Arc::clone(&rest_service);

	//simulate the bridge loop part
	let bridge_loop_future = tokio::spawn(async move {
		loop {
			if let Some(oneshot_tx) = health_rx.recv().await {
				if let Err(err) = oneshot_tx.send("OK".to_string()) {
					tracing::warn!("Heal check oneshot channel closed abnormally :{err:?}");
				}
			}
		}
	});

	let rest_service_future = tokio::spawn(async move {
		let _ = rest_service_for_task.run_service().await;
	});

	// Create the test client with the routes
	let client = TestClient::new(rest_service.create_routes());

	// Test the /health endpoint
	let response = client.get("/health").send().await;
	response.assert_status_is_ok();

	// Abort the REST service task
	rest_service_future.abort();
	bridge_loop_future.abort();

	Ok(())
}
