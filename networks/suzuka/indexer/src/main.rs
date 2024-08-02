use processor::IndexerGrpcProcessorConfig;
use server_framework::RunnableConfig;
use std::io::Write;

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

	let indexer_grpc_data_service_address = format!(
		"http://{}:{}",
		maptos_config.indexer.maptos_indexer_grpc_listen_hostname,
		maptos_config.indexer.maptos_indexer_grpc_listen_port
	);
	tracing::info!(
		"Connecting to indexer gRPC server at: {}",
		indexer_grpc_data_service_address.clone()
	);

	// let config = IndexerGrpcProcessorConfig {
	// 	processor_config: ProcessorConfig::DefaultProcessor,
	// 	postgres_connection_string: maptos_config.indexer_processor.postgres_connection_string,
	// 	indexer_grpc_data_service_address: indexer_grpc_data_service_address.parse()?,
	// 	grpc_http2_config: IndexerGrpcHttp2Config {
	// 		/// Indexer GRPC http2 ping interval in seconds. Defaults to 30.
	// 		/// Tonic ref: https://docs.rs/tonic/latest/tonic/transport/channel/struct.Endpoint.html#method.http2_keep_alive_interval
	// 		indexer_grpc_http2_ping_interval_in_secs: 60,
	// 		/// Indexer GRPC http2 ping timeout in seconds. Defaults to 10.
	// 		indexer_grpc_http2_ping_timeout_in_secs: 10,
	// 		/// Seconds before timeout for grpc connection.
	// 		indexer_grpc_connection_timeout_secs: 10,
	// 	},
	// 	auth_token: maptos_config.indexer_processor.indexer_processor_auth_token,
	// 	// Version to start indexing from
	// 	starting_version: None,
	// 	// Version to end indexing at
	// 	ending_version: None,
	// 	// Number of tasks waiting to pull transaction batches from the channel and process them
	// 	number_concurrent_processing_tasks: None,
	// 	// Size of the pool for writes/reads to the DB. Limits maximum number of queries in flight
	// 	db_pool_size: None,
	// 	// Maximum number of batches "missing" before we assume we have an issue with gaps and abort
	// 	gap_detection_batch_size: IndexerGrpcProcessorConfig::default_gap_detection_batch_size(),
	// 	// Maximum number of batches "missing" before we assume we have an issue with gaps and abort
	// 	parquet_gap_detection_batch_size:
	// 		IndexerGrpcProcessorConfig::default_gap_detection_batch_size(),
	// 	// Number of protobuff transactions to send per chunk to the processor tasks
	// 	pb_channel_txn_chunk_size: IndexerGrpcProcessorConfig::default_pb_channel_txn_chunk_size(),
	// 	// Number of rows to insert, per chunk, for each DB table. Default per table is ~32,768 (2**16/2)
	// 	per_table_chunk_sizes: AHashMap::new(),
	// 	enable_verbose_logging: None,
	// 	grpc_response_item_timeout_in_secs:
	// 		IndexerGrpcProcessorConfig::default_grpc_response_item_timeout_in_secs(),
	// 	transaction_filter: TransactionFilter::default(),
	// 	// String vector for deprecated tables to skip db writes
	// 	deprecated_tables: HashSet::new(),
	// };

	//create config file
	let indexer_config_content = format!(
		"processor_config:
  type: default_processor
postgres_connection_string: {}/postgres
indexer_grpc_data_service_address: {}
indexer_grpc_http2_ping_interval_in_secs: 60
indexer_grpc_http2_ping_timeout_in_secs: 10
auth_token: \"{}\"",
		maptos_config.indexer_processor.postgres_connection_string,
		indexer_grpc_data_service_address,
		maptos_config.indexer_processor.indexer_processor_auth_token,
	);

	let indexer_config_path = dot_movement.get_path().join("indexer_config.yaml");
	let mut output_file = std::fs::File::create(&indexer_config_path)?;
	write!(output_file, "{}", indexer_config_content)?;

	let indexer_config =
		server_framework::load::<IndexerGrpcProcessorConfig>(&indexer_config_path)?;

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
		.block_on(async move { indexer_config.run().await })
}
