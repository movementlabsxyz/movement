use processor::IndexerGrpcProcessorConfig;
use server_framework::RunnableConfig;
use std::io::Write;
use tokio::task::JoinSet;
use tokio::time::Duration;

mod service;

const RUNTIME_WORKER_MULTIPLIER: usize = 2;

fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let maptos_config =
		dot_movement.try_get_config_from_json::<maptos_execution_util::config::Config>()?;

	let health_check_url = format!(
		"{}:{}",
		maptos_config.indexer.maptos_indexer_grpc_healthcheck_hostname,
		maptos_config.indexer.maptos_indexer_grpc_healthcheck_port
	);

	let default_indexer_config = build_processor_conf("default_processor", &maptos_config)?;
	let usertx_indexer_config = build_processor_conf("user_transaction_processor", &maptos_config)?;
	let accounttx_indexer_config =
		build_processor_conf("account_transactions_processor", &maptos_config)?;
	let coin_indexer_config = build_processor_conf("coin_processor", &maptos_config)?;
	let event_indexer_config = build_processor_conf("events_processor", &maptos_config)?;
	let fungible_indexer_config = build_processor_conf("fungible_asset_processor", &maptos_config)?;
	let txmeta_indexer_config =
		build_processor_conf("transaction_metadata_processor", &maptos_config)?;

	// Token processor
	let activate_tokes: bool = std::env::var("ACTIVATE_TOKEN_INDEXING")
		.map(|t| t.parse().unwrap_or(true))
		.unwrap_or(true);
	let token_configs = if activate_tokes {
		let token_indexer_config = build_processor_conf(
			"token_processor
  nft_points_contract: null",
			&maptos_config,
		)?;

		let tokenv2_indexer_config = build_processor_conf(
			"token_v2_processor
  query_retries: 5",
			&maptos_config,
		)?;
		Some((token_indexer_config, tokenv2_indexer_config))
	} else {
		None
	};

	let num_cpus = num_cpus::get();
	let worker_threads = (num_cpus * RUNTIME_WORKER_MULTIPLIER).max(16);
	println!(
		"[Processor] Starting processor tokio runtime: num_cpus={}, worker_threads={}",
		num_cpus, worker_threads
	);

	let mut builder = tokio::runtime::Builder::new_multi_thread();
	let ret: Result<(), anyhow::Error> = builder
		.disable_lifo_slot()
		.enable_all()
		.worker_threads(worker_threads)
		.build()
		.unwrap()
		.block_on({
			async move {
				// Test the Grpc connection.
				// The gRpc connection can fail because the Movement-node is started but the port is still not open.
				// If the connection fail wait and retry.
				test_grpc_connection(&maptos_config).await?;

				let mut set = JoinSet::new();
				set.spawn(async move { crate::service::run_service(health_check_url).await });
				set.spawn(async move { default_indexer_config.run().await });
				//wait all the migration is done.
				tokio::time::sleep(Duration::from_secs(12)).await;
				set.spawn(async move { usertx_indexer_config.run().await });
				set.spawn(async move { accounttx_indexer_config.run().await });
				set.spawn(async move { coin_indexer_config.run().await });
				set.spawn(async move { event_indexer_config.run().await });
				set.spawn(async move { fungible_indexer_config.run().await });
				set.spawn(async move { txmeta_indexer_config.run().await });
				if let Some((token_indexer_config, tokenv2_indexer_config)) = token_configs {
					set.spawn(async move { token_indexer_config.run().await });
					set.spawn(async move { tokenv2_indexer_config.run().await });
				}

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

fn build_processor_conf(
	processor_name: &str,
	maptos_config: &maptos_execution_util::config::Config,
) -> Result<IndexerGrpcProcessorConfig, anyhow::Error> {
	let indexer_grpc_data_service_address = build_grpc_url(maptos_config);

	let default_sleep_time_between_request: u64 = std::env::var("SLEEP_TIME_BETWEEN_REQUEST_MS")
		.map(|t| t.parse().unwrap_or(10))
		.unwrap_or(10);

	// If the starting version is not defined, don't put a default value in the conf.
	let starting_version_entry = std::env::var("INDEXER_STARTING_VERSION")
		.map(|t| t.parse().unwrap_or(0))
		.map(|t| format!("starting_version: {}", t))
		.unwrap_or(String::new());

	//create config file
	let indexer_config_content = format!(
		"processor_config:
  type: {}
postgres_connection_string: {}/postgres
indexer_grpc_data_service_address: {}
indexer_grpc_http2_ping_interval_in_secs: {}
indexer_grpc_http2_ping_timeout_in_secs: {}
auth_token: \"{}\"
default_sleep_time_between_request: {}
{}",
		processor_name,
		maptos_config.indexer_processor.postgres_connection_string,
		indexer_grpc_data_service_address,
		maptos_config.indexer.maptos_indexer_grpc_inactivity_timeout,
		maptos_config.indexer.maptos_indexer_grpc_inactivity_ping_interval,
		maptos_config.indexer_processor.indexer_processor_auth_token,
		default_sleep_time_between_request,
		starting_version_entry,
	);

	//let indexer_config_path = dot_movement.get_path().join("indexer_config.yaml");
	let mut output_file = tempfile::NamedTempFile::new()?;
	write!(output_file, "{}", indexer_config_content)?;

	let mut indexer_config =
		server_framework::load::<IndexerGrpcProcessorConfig>(&output_file.path().to_path_buf())?;

	// Leave here for debug purpose. Will be removed later.
	// Use to print the generated config, to have an example when activating a new processor.
	// indexer_config.processor_config = ProcessorConfig::TokenV2Processor(TokenV2ProcessorConfig {
	// 	query_retries: 5,
	// 	query_retry_delay_ms: 100,
	// });

	// let yaml = serde_yaml::to_string(&indexer_config)?;
	// println!("{yaml}",);

	Ok(indexer_config)
}

use reqwest::Client as HttpClient;

async fn test_grpc_connection(
	maptos_config: &maptos_execution_util::config::Config,
) -> Result<(), anyhow::Error> {
	let indexer_grpc_data_service_address = build_grpc_url(maptos_config);

	let client = HttpClient::builder()
		.http2_prior_knowledge() // Enforce HTTP/2 for gRpc
		.timeout(Duration::from_secs(10))
		.build()?;

	let mut retry = 0;
	while retry < 5 {
		let response = client
			.get(&indexer_grpc_data_service_address)
			.header("Content-Type", "application/grpc")
			.send()
			.await;

		match response {
			Ok(resp) => {
				let status = resp.status();
				let body = resp.text().await?;
				println!("Received response: {} {:?}", status, body);
				if status.is_success() {
					break;
				} else {
					tracing::info!("GRpc server return a bad status: {:?}. Retrying...", status);
					tokio::time::sleep(Duration::from_secs(1)).await; // Wait before retrying
				}
			}
			Err(err) => {
				tracing::info!("Failed to connect to the gRp server: {:?}. Retrying...", err);
				tokio::time::sleep(Duration::from_secs(1)).await; // Wait before retrying
			}
		}
		retry += 1;
	}

	if retry == 5 {
		Err(anyhow::anyhow!(
			"Faild to connect to the Grpc server : {indexer_grpc_data_service_address}"
		))
	} else {
		Ok(())
	}
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
