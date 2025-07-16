use aptos_indexer_processor_sdk::server_framework::RunnableConfig;
use processor::config::indexer_processor_config::IndexerProcessorConfig;
use tokio::task::JoinSet;
use tokio::time::Duration;

mod service;

const RUNTIME_WORKER_MULTIPLIER: usize = 2;
const DEFAULT_PROCESSOR_NAMES: &[&str] = &["default_processor"];

fn main() -> Result<(), anyhow::Error> {
	init_logger(None);

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let maptos_config =
		dot_movement.try_get_config_from_json::<maptos_execution_util::config::Config>()?;

	let health_check_url = format!(
		"{}:{}",
		maptos_config.indexer.maptos_indexer_grpc_healthcheck_hostname,
		maptos_config.indexer.maptos_indexer_grpc_healthcheck_port
	);

	let processor_configs = get_processor_configs(&maptos_config)?;
	let runtime = get_movement_runtime_builder();

	let ret: Result<(), anyhow::Error> = runtime.block_on({
		async move {
			let mut set = JoinSet::new();
			// TODO: use generic health check server.

			let mut port_offset = 0;
			for mut config in processor_configs {
				let processor_name = config.server_config.processor_config.name().to_string();
				config.server_config.processor_config.health_check_port += port_offset;

				set.spawn(async move { config.run().await });
				port_offset += 1;
			}
			//wait all the migration is done.
			tokio::time::sleep(Duration::from_secs(12)).await;

			while let Some(res) = set.join_next().await {
				tracing::error!("An Error occurs during indexer execution: {res:?}");
				// If a processor break to avoid data inconsistency between processor
				break;
			}
			set.shutdown().await;
			Err(anyhow::anyhow!("At least One indexer processor failed. Exit"))
		}
	});
	if let Err(err) = ret {
		tracing::error!("Indexer execution failed: {err}");
		std::process::exit(1);
	} else {
		std::process::exit(1);
	}
}

fn build_processor_conf_v2(
	processor_name: &str,
	maptos_config: &maptos_execution_util::config::Config,
) -> Result<IndexerProcessorConfig, anyhow::Error> {
	let indexer_processor_raw = format!(
		r#"
health_check_port: 8085
server_config:
  processor_config:
    type: {}
    channel_size: 100
  transaction_stream_config:
    indexer_grpc_data_service_address: "{}"
    auth_token: "{}"
    request_name_header: "{}"
  processor_mode:
    type: "default"
  db_config:
    type: postgres_config
    connection_string: {}
	"#,
		processor_name,
		build_grpc_url(maptos_config),
		maptos_config.indexer_processor.indexer_processor_auth_token,
		"".to_string(),
		maptos_config.indexer_processor.postgres_connection_string,
	);
	let config = serde_yaml::from_str::<IndexerProcessorConfig>(&indexer_processor_raw)?;
	// TODO: Fix the starting version.
	Ok(config)
}

// fn build_processor_conf(
// 	processor_name: &str,
// 	maptos_config: &maptos_execution_util::config::Config,
// ) -> Result<IndexerGrpcProcessorConfig, anyhow::Error> {
// 	let indexer_grpc_data_service_address = build_grpc_url(maptos_config);

// 	let default_sleep_time_between_request: u64 = std::env::var("SLEEP_TIME_BETWEEN_REQUEST_MS")
// 		.map(|t| t.parse().unwrap_or(10))
// 		.unwrap_or(10);

// 	//create config file
// 	let mut indexer_config_content = format!(
// 		"processor_config:
//   type: {}
// postgres_connection_string: {}
// indexer_grpc_data_service_address: {}
// indexer_grpc_http2_ping_interval_in_secs: {}
// indexer_grpc_http2_ping_timeout_in_secs: {}
// auth_token: \"{}\"
// default_sleep_time_between_request: {}",
// 		processor_name,
// 		maptos_config.indexer_processor.postgres_connection_string,
// 		indexer_grpc_data_service_address,
// 		maptos_config.indexer.maptos_indexer_grpc_inactivity_timeout,
// 		maptos_config.indexer.maptos_indexer_grpc_inactivity_ping_interval,
// 		maptos_config.indexer_processor.indexer_processor_auth_token,
// 		default_sleep_time_between_request,
// 	);

// 	// If the starting version is not defined, don't put a default value in the conf.
// 	if let Ok(start_version) = std::env::var("INDEXER_STARTING_VERSION") {
// 		if let Ok(start_version) = start_version.parse::<u64>() {
// 			indexer_config_content.push('\n');
// 			indexer_config_content.push_str(&format!("starting_version: {}", start_version));
// 		}
// 	}

// 	//let indexer_config_path = dot_movement.get_path().join("indexer_config.yaml");
// 	let mut output_file = tempfile::NamedTempFile::new()?;
// 	write!(output_file, "{}", indexer_config_content)?;

// 	let indexer_config =
// 		server_framework::load::<IndexerGrpcProcessorConfig>(&output_file.path().to_path_buf())?;

// 	// Leave here for debug purpose. Will be removed later.
// 	// Use to print the generated config, to have an example when activating a new processor.
// 	// indexer_config.processor_config = ProcessorConfig::TokenV2Processor(TokenV2ProcessorConfig {
// 	// 	query_retries: 5,
// 	// 	query_retry_delay_ms: 100,
// 	// });

// 	// let yaml = serde_yaml::to_string(&indexer_config)?;
// 	// println!("{yaml}",);

// 	Ok(indexer_config)
// }

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

fn init_logger(log_level: Option<String>) {
	use tracing_subscriber::EnvFilter;
	let log_level = log_level.unwrap_or("info".to_string());
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
		)
		.init();
}

fn get_movement_runtime_builder() -> tokio::runtime::Runtime {
	let num_cpus = num_cpus::get();
	let worker_threads = (num_cpus * RUNTIME_WORKER_MULTIPLIER).max(16);
	println!(
		"[Processor] Starting processor tokio runtime: num_cpus={}, worker_threads={}",
		num_cpus, worker_threads
	);

	let mut builder = tokio::runtime::Builder::new_multi_thread();
	builder
		.disable_lifo_slot()
		.enable_all()
		.worker_threads(worker_threads)
		.build()
		.unwrap()
}

fn get_processor_configs(
	maptos_config: &maptos_execution_util::config::Config,
) -> Result<Vec<IndexerProcessorConfig>, anyhow::Error> {
	let mut configs = vec![];
	if maptos_config.indexer_processor.processor_names.is_empty() {
		for processor_name in DEFAULT_PROCESSOR_NAMES.iter() {
			let config = build_processor_conf_v2(processor_name, maptos_config)?;
			configs.push(config);
		}
	} else {
		for processor_name in maptos_config.indexer_processor.processor_names.iter() {
			let config = build_processor_conf_v2(processor_name, maptos_config)?;
			configs.push(config);
		}
	}

	tracing::info!("Total number of processor configs: {}", configs.len());

	Ok(configs)
}
