use bridge_config::Config;
use bridge_service::rest::BridgeRest;
use poem::test::TestClient;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_rest_service_health_endpoint() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let mock_config = Config::default();
	let rest_service = Arc::new(BridgeRest::new(&mock_config.movement)?);
	let rest_service_for_task = Arc::clone(&rest_service);

	let rest_service_future = tokio::spawn(async move {
		rest_service_for_task.run_service().await?;
	});

	let client = TestClient::new(rest_service.create_routes());

	let response = client.get("/health").send().await;
	response.assert_status_is_ok();

	rest_service_future.abort();
}
