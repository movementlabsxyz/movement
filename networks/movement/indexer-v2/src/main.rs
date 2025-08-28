use aptos_indexer_processor_sdk::server_framework::RunnableConfig;
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_config::Config;
use movement_health::run_service;
use movement_tracing::simple_metrics::start_metrics_server;
use processor_v2::config::indexer_processor_config::IndexerProcessorConfig;

const RUNTIME_WORKER_MULTIPLIER: usize = 2;

fn main() -> Result<(), anyhow::Error> {
	init_logger();

	let runtime = get_maptos_runtime();

	runtime.block_on(async move {
		let config = load_maptos_config().await.expect("Failed to load maptos config");
		let indexer_config = config.execution_config.maptos_config.indexer_processor_v2;
		let runnable_processor_config: IndexerProcessorConfig = indexer_config.clone().into();
		let metrics_config = indexer_config.metrics_config.clone();
		let metrics_handle = tokio::spawn(async move {
			let res =
				start_metrics_server(metrics_config.listen_hostname, metrics_config.listen_port)
					.await;
			tracing::info!("Metrics server started: {:?}", res);
			res
		});

		let health_config = indexer_config.health_config.clone();
		let health_handle = tokio::spawn(async move {
			let res = run_service(health_config.hostname, health_config.port).await;
			tracing::info!("Health server started: {:?}", res);
			res
		});

		let processor_handle = tokio::spawn(async move {
			let res = runnable_processor_config.run().await;
			tracing::info!("Indexer processor started: {:?}", res);
			res
		});
		tracing::info!("Indexer v2 stack started.");

		tokio::select! {
			metrics_handle = metrics_handle => {
				tracing::error!("Metrics server exited abnormally: {:?}", metrics_handle);
			}
			health_handle = health_handle => {
				tracing::error!("Health server exited abnormally: {:?}", health_handle);
			}
			processor_handle = processor_handle => {
				tracing::error!("Indexer processor exited abnormally: {:?}", processor_handle);
			}
		}
		tracing::info!("Indexer v2 stack exited normally.");
	});
	panic!("Indexer v2 stack exited abnormally.");
}

fn init_logger() {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();
}

fn get_maptos_runtime() -> tokio::runtime::Runtime {
	let num_cpus = num_cpus::get();
	let worker_threads = (num_cpus * RUNTIME_WORKER_MULTIPLIER).max(16);
	tracing::info!(
		"[Processor] Starting processor tokio runtime: num_cpus={}, worker_threads={}",
		num_cpus,
		worker_threads
	);

	let mut builder = tokio::runtime::Builder::new_multi_thread();
	let runtime =
		match builder.disable_lifo_slot().enable_all().worker_threads(worker_threads).build() {
			Ok(runtime) => runtime,
			Err(e) => {
				tracing::error!("Error building tokio runtime: {}", e);
				panic!("Error building tokio runtime for indexer-v2.");
			}
		};
	runtime
}

async fn load_maptos_config() -> anyhow::Result<Config> {
	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;

	// Load config
	let config = {
		let config_file = dot_movement.try_get_or_create_config_file().await?;
		let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);
		godfig.try_wait_for_ready().await
	}?;

	Ok(config)
}
