// pub mod alice_bob;
// pub mod indexer_stream;
// use std::str::FromStr;
// use url::Url;
use aptos_protos::indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest};
use futures::StreamExt;
use once_cell::sync::Lazy;

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

static INDEXER_URL: Lazy<String> = Lazy::new(|| {
	let indexer_connection_hostname = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_indexer_grpc_connection_hostname
		.clone();
	let indexer_connection_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_indexer_grpc_connection_port
		.clone();

	let indexer_connection_url =
		format!("http://{}:{}", indexer_connection_hostname, indexer_connection_port);

	indexer_connection_url
});

#[tokio::test]
async fn test_example_indexer_stream() -> Result<(), anyhow::Error> {
	/*let channel = tonic::transport::Channel::from_shared(
		INDEXER_URL.to_string(),
	).expect(
		"[Parser] Failed to build GRPC channel, perhaps because the data service URL is invalid",
	);*/

	let mut client = RawDataClient::connect(INDEXER_URL.as_str()).await?;

	let request = GetTransactionsRequest {
		starting_version: Some(1),
		transactions_count: Some(10),
		batch_size: Some(100),
	};

	let mut stream = client.get_transactions(request).await?.into_inner();

	for _ in 1..10 {
		let response = stream.next().await;
		println!("{:?}", response);
	}

	Ok(())
}
