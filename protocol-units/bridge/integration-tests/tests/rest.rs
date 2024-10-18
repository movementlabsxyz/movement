#[tokio::test]
async fn test_rest_service_health_endpoint() {
	use poem::test::TestClient;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let mock_config = Config::default();

	let rest_service = BridgeRest::new(mock_config.movement.clone()).unwrap();

	let rest_service_future = tokio::spawn(async move {
		rest_service.run_service().await.unwrap();
	});

	sleep(Duration::from_millis(500)).await;

	let client = TestClient::new(rest_service.create_routes());

	let response = client.get("/health").send().await;
	assert!(response.status().is_success(), "Health endpoint failed");
	assert_eq!(response.text().await.unwrap(), "OK");

	rest_service_future.abort();
}
