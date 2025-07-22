use aptos_indexer_processor_sdk::server_framework::RunnableConfig;
use maptos_execution_util::config::Config;
use movement_health::run_service;
use movement_tracing::simple_metrics::start_metrics_server;
use processor_v2::config::indexer_processor_config::IndexerProcessorConfig;
use tokio::task::JoinSet;

const RUNTIME_WORKER_MULTIPLIER: usize = 2;

fn main() -> Result<(), anyhow::Error> {
	init_logger();

	let runtime = get_maptos_runtime();
	let maptos_config = load_maptos_config()?;
	let runnable_processor_config: IndexerProcessorConfig =
		maptos_config.indexer_processor_v2.clone().into();

	runtime.block_on(async move {
		let mut set = JoinSet::new();

		set.spawn(async move {
			tracing::info!("Starting metrics server");
			start_metrics_server(
				maptos_config.indexer_processor_v2.metrics_config.listen_hostname,
				maptos_config.indexer_processor_v2.metrics_config.listen_port,
			)
			.await
		});
		set.spawn(async move {
			tracing::info!("Starting health server");
			run_service(
				maptos_config.indexer_processor_v2.health_config.hostname,
				maptos_config.indexer_processor_v2.health_config.port,
			)
			.await
		});
		set.spawn(async move {
			tracing::info!("Starting indexer processor");
			runnable_processor_config.run().await
		});

		//wait all the migration is done.
		set.join_all().await;
	});
	Ok(())
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

fn load_maptos_config() -> anyhow::Result<Config> {
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let maptos_config = dot_movement.try_get_config_from_json::<Config>()?;
	Ok(maptos_config)
}
