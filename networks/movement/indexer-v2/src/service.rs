use anyhow::Result;
use aptos_indexer_processor_sdk::server_framework::ServerArgs;
use processor::config::indexer_processor_config::IndexerProcessorConfig;
use std::io::Write;
use tempfile::NamedTempFile;

pub struct MovementIndexerV2;

impl MovementIndexerV2 {
	/// Create and run the Movement indexer v2
	pub async fn run() -> Result<()> {
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let maptos_config =
			dot_movement.try_get_config_from_json::<maptos_execution_util::config::Config>()?;

		// Create a temporary config file for the v2 processor
		let config_content = create_v2_config(&maptos_config)?;
		let mut config_file = NamedTempFile::new()?;
		write!(config_file, "{}", config_content)?;

		// Set the config file path as an environment variable for the SDK
		std::env::set_var("CONFIG_FILE", config_file.path().to_str().unwrap());

		let args = ServerArgs::parse();
		args.run::<IndexerProcessorConfig>(tokio::runtime::Handle::current()).await
	}
}

fn create_v2_config(maptos_config: &maptos_execution_util::config::Config) -> Result<String> {
	let indexer_grpc_data_service_address = build_grpc_url(maptos_config);

	// Get starting version from environment or use 0
	let starting_version = std::env::var("INDEXER_STARTING_VERSION")
		.map(|v| v.parse::<u64>().unwrap_or(0))
		.unwrap_or(0);

	// Create config using the new v2 format
	let config_content = format!(
		r#"health_check_port: 8086
server_config:
  processor_config:
    type: default_processor
    channel_size: 100
  transaction_stream_config:
    indexer_grpc_data_service_address: "{}"
    auth_token: "{}"
    request_name_header: "movement_indexer_v2"
    indexer_grpc_http2_ping_interval_in_secs: {}
    indexer_grpc_http2_ping_timeout_in_secs: {}
    indexer_grpc_reconnection_timeout_secs: 60
    indexer_grpc_response_item_timeout_secs: 60
  processor_mode:
    type: "default"
    initial_starting_version: {}
  db_config:
    type: postgres_config
    connection_string: "{}""#,
		indexer_grpc_data_service_address,
		maptos_config.indexer_processor.indexer_processor_auth_token,
		maptos_config.indexer.maptos_indexer_grpc_inactivity_ping_interval,
		maptos_config.indexer.maptos_indexer_grpc_inactivity_timeout,
		starting_version,
		maptos_config.indexer_processor.postgres_connection_string,
	);

	Ok(config_content)
}

fn build_grpc_url(maptos_config: &maptos_execution_util::config::Config) -> String {
	let indexer_grpc_data_service_address = format!(
		"http://{}:{}",
		maptos_config.indexer.maptos_indexer_grpc_listen_hostname,
		maptos_config.indexer.maptos_indexer_grpc_listen_port
	);
	tracing::info!(
		"Connecting to indexer gRPC server at: {}",
		indexer_grpc_data_service_address.clone()
	);
	indexer_grpc_data_service_address
}
