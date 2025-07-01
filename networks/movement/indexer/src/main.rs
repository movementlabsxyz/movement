use aptos_indexer_processor_sdk::aptos_indexer_transaction_stream::TransactionStreamConfig;
use aptos_indexer_processor_sdk::server_framework::RunnableConfig;
use processor::config::db_config::{DbConfig, PostgresConfig};
use processor::config::indexer_processor_config::IndexerProcessorConfig;
use processor::config::processor_config::{DefaultProcessorConfig, ProcessorConfig};
use processor::config::processor_mode::{BootStrapConfig, ProcessorMode};
use processor::processors::ans::ans_processor::AnsProcessorConfig;
use processor::processors::token_v2::token_v2_processor::TokenV2ProcessorConfig;
use std::env;
use std::fs;
use std::path::Path;
use tokio::task::JoinSet;
//use tokio::time::Duration;
use url::Url;

mod service;

// const RUNTIME_WORKER_MULTIPLIER: usize = 2;

async fn ensure_migrations_run() -> Result<(), anyhow::Error> {
	let lock_file = "/tmp/movement_indexer_migrations.lock";

	// Check if migrations have already been run
	if Path::new(lock_file).exists() {
		tracing::info!("Migrations already completed, skipping...");
		return Ok(());
	}

	// Create lock file to indicate migrations are running
	fs::write(lock_file, "migrations_in_progress")?;

	// Wait a bit to ensure any other processes see the lock
	tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

	// Update lock file to indicate completion
	fs::write(lock_file, "migrations_completed")?;

	tracing::info!("Migration lock file created");
	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	tracing_subscriber::fmt::init();

	let maptos_config =
		dot_movement.try_get_config_from_json::<maptos_execution_util::config::Config>()?;

	// MIGRATE_ONLY mode: run only one processor for migrations, then exit
	if env::var("MIGRATE_ONLY").ok().as_deref() == Some("1") {
		let migration_config = build_processor_conf("default_processor", &maptos_config)?;
		migration_config.run().await?;
		println!("Migrations completed.");
		return Ok(());
	}

	// Ensure migrations are handled properly
	ensure_migrations_run().await?;

	let mut set = JoinSet::new();

	// Create configs for each processor
	let default_indexer_config = build_processor_conf("default_processor", &maptos_config)?;
	let usertx_indexer_config = build_processor_conf("user_transaction_processor", &maptos_config)?;
	let accounttx_indexer_config =
		build_processor_conf("account_transactions_processor", &maptos_config)?;
	let coin_indexer_config = build_processor_conf("fungible_asset_processor", &maptos_config)?;
	let event_indexer_config = build_processor_conf("events_processor", &maptos_config)?;
	let fungible_indexer_config = build_processor_conf("fungible_asset_processor", &maptos_config)?;
	let txmeta_indexer_config =
		build_processor_conf("transaction_metadata_processor", &maptos_config)?;
	let token_indexer_config = build_processor_conf("token_processor", &maptos_config)?;
	let tokenv2_indexer_config = build_processor_conf("token_v2_processor", &maptos_config)?;
	let ans_indexer_config = build_processor_conf("ans_processor", &maptos_config)?;

	// Spawn processors based on configuration
	set.spawn(async move { default_indexer_config.run().await });
	set.spawn(async move { usertx_indexer_config.run().await });
	set.spawn(async move { accounttx_indexer_config.run().await });
	set.spawn(async move { coin_indexer_config.run().await });
	set.spawn(async move { event_indexer_config.run().await });
	set.spawn(async move { fungible_indexer_config.run().await });
	set.spawn(async move { txmeta_indexer_config.run().await });
	set.spawn(async move { token_indexer_config.run().await });
	set.spawn(async move { tokenv2_indexer_config.run().await });
	set.spawn(async move { ans_indexer_config.run().await });

	// Wait for all processors to complete
	while let Some(result) = set.join_next().await {
		match result {
			Ok(Ok(())) => tracing::info!("Processor completed successfully"),
			Ok(Err(e)) => tracing::error!("Processor failed: {:?}", e),
			Err(e) => tracing::error!("Task join error: {:?}", e),
		}
	}

	Ok(())
}

fn build_processor_conf(
	processor_name: &str,
	maptos_config: &maptos_execution_util::config::Config,
) -> Result<IndexerProcessorConfig, anyhow::Error> {
	let indexer_grpc_data_service_address = build_grpc_url(maptos_config);

	// Create processor config based on processor name
	let processor_config = match processor_name {
		"default_processor" => ProcessorConfig::DefaultProcessor(DefaultProcessorConfig::default()),
		"user_transaction_processor" => {
			ProcessorConfig::UserTransactionProcessor(DefaultProcessorConfig::default())
		}
		"account_transactions_processor" => {
			ProcessorConfig::AccountTransactionsProcessor(DefaultProcessorConfig::default())
		}
		"events_processor" => ProcessorConfig::EventsProcessor(DefaultProcessorConfig::default()),
		"fungible_asset_processor" => {
			ProcessorConfig::FungibleAssetProcessor(DefaultProcessorConfig::default())
		}
		"transaction_metadata_processor" => {
			ProcessorConfig::MonitoringProcessor(DefaultProcessorConfig::default())
		}
		"token_processor" => ProcessorConfig::DefaultProcessor(DefaultProcessorConfig::default()),
		"token_v2_processor" => ProcessorConfig::TokenV2Processor(TokenV2ProcessorConfig {
			default_config: DefaultProcessorConfig::default(),
			query_retries: 3,
			query_retry_delay_ms: 1000,
		}),
		"ans_processor" => ProcessorConfig::AnsProcessor(AnsProcessorConfig {
			default: DefaultProcessorConfig::default(),
			ans_v1_primary_names_table_handle: "0x1::ans::AnsPrimaryNameV2".to_string(),
			ans_v1_name_records_table_handle: "0x1::ans::AnsLookupV2".to_string(),
			ans_v2_contract_address: "0x1::ans".to_string(),
		}),
		_ => ProcessorConfig::DefaultProcessor(DefaultProcessorConfig::default()),
	};

	// Create transaction stream config
	let starting_version = std::env::var("INDEXER_STARTING_VERSION")
		.map(|v| v.parse::<u64>().unwrap_or(0))
		.unwrap_or(0);

	let transaction_stream_config = TransactionStreamConfig {
		starting_version: Some(starting_version),
		request_ending_version: None,
		indexer_grpc_data_service_address: indexer_grpc_data_service_address.parse::<Url>()?,
		auth_token: maptos_config.indexer_processor.indexer_processor_auth_token.clone(),
		request_name_header: processor_name.to_string(),
		additional_headers: Default::default(),
		indexer_grpc_http2_ping_interval_secs: 30,
		indexer_grpc_http2_ping_timeout_secs: 10,
		indexer_grpc_reconnection_timeout_secs: 60,
		indexer_grpc_response_item_timeout_secs: 60,
		indexer_grpc_reconnection_max_retries: 3,
		transaction_filter: None,
	};

	// Create DB config
	let db_config = DbConfig::PostgresConfig(PostgresConfig {
		connection_string: maptos_config.indexer_processor.postgres_connection_string.clone(),
		db_pool_size: 10,
	});

	// Create processor mode
	let processor_mode =
		ProcessorMode::Default(BootStrapConfig { initial_starting_version: starting_version });

	Ok(IndexerProcessorConfig {
		processor_config,
		transaction_stream_config,
		db_config,
		processor_mode,
	})
}

// use reqwest::Client as HttpClient;

// async fn test_grpc_connection(
// 	maptos_config: &maptos_execution_util::config::Config,
// ) -> Result<(), anyhow::Error> {
// 	let indexer_grpc_data_service_address = build_grpc_url(maptos_config);

// 	let client = HttpClient::builder()
// 		.http2_prior_knowledge() // Enforce HTTP/2 for gRpc
// 		.timeout(Duration::from_secs(10))
// 		.build()?;

// 	let mut retry = 0;
// 	while retry < 5 {
// 		let response = client
// 			.get(&indexer_grpc_data_service_address)
// 			.header("Content-Type", "application/grpc")
// 			.send()
// 			.await;

// 		match response {
// 			Ok(resp) => {
// 				let status = resp.status();
// 				let body = resp.text().await?;
// 				println!("Received response: {} {:?}", status, body);
// 				if status.is_success() {
// 					break;
// 				} else {
// 					tracing::info!("GRpc server return a bad status: {:?}. Retrying...", status);
// 					tokio::time::sleep(Duration::from_secs(1)).await; // Wait before retrying
// 				}
// 			}
// 			Err(err) => {
// 				tracing::info!("Failed to connect to the gRp server: {:?}. Retrying...", err);
// 				tokio::time::sleep(Duration::from_secs(1)).await; // Wait before retrying
// 			}
// 		}
// 		retry += 1;
// 	}

// 	if retry == 5 {
// 		Err(anyhow::anyhow!(
// 			"Faild to connect to the Grpc server : {indexer_grpc_data_service_address}"
// 		))
// 	} else {
// 		Ok(())
// 	}
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
