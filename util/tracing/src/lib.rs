use aptos_config::config::{NodeConfig, RoleType};
use aptos_logger::{info, warn};
use aptos_telemetry;
use aptos_types::chain_id::ChainId;
use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use prometheus::{Counter, Encoder, Gauge};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time;
use warp::Filter;

// Create a default NodeConfig for telemetry
static DEFAULT_NODE_CONFIG: Lazy<NodeConfig> = Lazy::new(|| {
	let mut config = NodeConfig::default();
	config.base.role = RoleType::FullNode;
	config.base.data_dir = "/tmp/aptos".into();
	config
});

static DEFAULT_CHAIN_ID: Lazy<ChainId> = Lazy::new(|| ChainId::new(1));
static DEFAULT_BUILD_INFO: Lazy<BTreeMap<String, String>> = Lazy::new(BTreeMap::new);

// Global runtime handle
static RUNTIME: Lazy<Arc<Runtime>> =
	Lazy::new(|| Arc::new(Runtime::new().expect("Failed to create runtime")));

// Track if metrics have been initialized
static METRICS_INITIALIZED: AtomicBool = AtomicBool::new(false);

// Custom metrics collectors
lazy_static! {
	static ref CONSENSUS_METRICS: Arc<ConsensusMetrics> = Arc::new(ConsensusMetrics::new());
	static ref DB_METRICS: Arc<DbMetrics> = Arc::new(DbMetrics::new());
	static ref MEMPOOL_METRICS: Arc<MempoolMetrics> = Arc::new(MempoolMetrics::new());
	static ref TRANSACTION_METRICS: Arc<TransactionMetrics> = Arc::new(TransactionMetrics::new());
	static ref SYNC_METRICS: Arc<SyncMetrics> = Arc::new(SyncMetrics::new());
}

// Metrics collector that automatically collects metrics from aptos-core components
struct AptosMetricsCollector;

impl AptosMetricsCollector {
	fn new() -> Self {
		Self
	}

	fn collect_metrics(&self) {
		// Collect metrics from the global registry
		let metrics = prometheus::gather();

		for metric_family in metrics {
			let name = metric_family.get_name();
			info!("Found metric: {}", name);

			// Update our metrics based on the metric names from aptos-core
			match name {
				// Consensus metrics
				"movement_consensus_last_committed_version" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting consensus block height to: {}", value);
						CONSENSUS_METRICS.block_height.set(value);
					}
				}
				"movement_consensus_current_round" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting consensus round to: {}", value);
						CONSENSUS_METRICS.round.set(value);
					}
				}
				"movement_consensus_round_timing" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting consensus round timing to: {}", value);
						CONSENSUS_METRICS.round_timing.set(value);
					}
				}

				// Storage metrics
				"movement_storage_ledger_version" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting ledger version to: {}", value);
						DB_METRICS.db_size.set(value);
					}
				}

				// Mempool metrics
				"movement_mempool_size" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting mempool size to: {}", value);
						MEMPOOL_METRICS.txn_count.set(value);
					}
				}
				"movement_mempool_txn_processing_time" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting mempool txn processing time to: {}", value);
						MEMPOOL_METRICS.processing_time.set(value);
					}
				}
				"movement_mempool_broadcast_time" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting mempool broadcast time to: {}", value);
						MEMPOOL_METRICS.broadcast_time.set(value);
					}
				}
				"movement_mempool_pending_txns" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting pending txns count to: {}", value);
						MEMPOOL_METRICS.pending_txns.set(value);
					}
				}

				// Transaction metrics
				"movement_txn_retrieval_latency" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting txn retrieval latency to: {}", value);
						TRANSACTION_METRICS.retrieval_latency.set(value);
					}
				}
				"movement_txn_commit_latency" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting txn commit latency to: {}", value);
						TRANSACTION_METRICS.commit_latency.set(value);
					}
				}
				"movement_txn_save_latency" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting txn save latency to: {}", value);
						TRANSACTION_METRICS.save_latency.set(value);
					}
				}

				// State sync metrics
				"movement_state_sync_version" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_counter().get_value();
						info!("Setting synced version to: {}", value);
						SYNC_METRICS.synced_version.set(value);
					}
				}
				"movement_state_sync_progress" => {
					if let Some(metric) = metric_family.get_metric().first() {
						let value = metric.get_gauge().get_value();
						info!("Setting sync progress to: {}", value);
						SYNC_METRICS.sync_progress.set(value);
					}
				}
				_ => {}
			}
		}
	}
}

struct ConsensusMetrics {
	block_height: Gauge,
	round: Gauge,
	committed_txns: Counter,
	round_timing: Gauge,
}

impl ConsensusMetrics {
	fn new() -> Self {
		Self {
			block_height: Gauge::new(
				"movement_consensus_last_committed_version",
				"Last committed version",
			)
			.unwrap(),
			round: Gauge::new("movement_consensus_current_round", "Current consensus round")
				.unwrap(),
			committed_txns: Counter::new("movement_state_sync_version", "Synced version").unwrap(),
			round_timing: Gauge::new("movement_consensus_round_timing", "Consensus round timing")
				.unwrap(),
		}
	}
}

struct DbMetrics {
	db_size: Gauge,
}

impl DbMetrics {
	fn new() -> Self {
		Self { db_size: Gauge::new("movement_storage_ledger_version", "Ledger version").unwrap() }
	}
}

struct MempoolMetrics {
	txn_count: Gauge,
	processing_time: Gauge,
	broadcast_time: Gauge,
	pending_txns: Gauge,
}

impl MempoolMetrics {
	fn new() -> Self {
		Self {
			txn_count: Gauge::new("movement_mempool_size", "Mempool size").unwrap(),
			processing_time: Gauge::new(
				"movement_mempool_txn_processing_time",
				"Transaction processing time",
			)
			.unwrap(),
			broadcast_time: Gauge::new(
				"movement_mempool_broadcast_time",
				"Transaction broadcast time",
			)
			.unwrap(),
			pending_txns: Gauge::new("movement_mempool_pending_txns", "Pending transactions count")
				.unwrap(),
		}
	}
}

struct TransactionMetrics {
	retrieval_latency: Gauge,
	commit_latency: Gauge,
	save_latency: Gauge,
}

impl TransactionMetrics {
	fn new() -> Self {
		Self {
			retrieval_latency: Gauge::new(
				"movement_txn_retrieval_latency",
				"Transaction retrieval latency",
			)
			.unwrap(),
			commit_latency: Gauge::new("movement_txn_commit_latency", "Transaction commit latency")
				.unwrap(),
			save_latency: Gauge::new("movement_txn_save_latency", "Transaction save latency")
				.unwrap(),
		}
	}
}

struct SyncMetrics {
	synced_version: Gauge,
	sync_progress: Gauge,
}

impl SyncMetrics {
	fn new() -> Self {
		Self {
			synced_version: Gauge::new("movement_state_sync_version", "Synced version").unwrap(),
			sync_progress: Gauge::new("movement_state_sync_progress", "Sync progress").unwrap(),
		}
	}
}

fn register_custom_metrics() {
	// Only register metrics if they haven't been registered before
	if METRICS_INITIALIZED.load(Ordering::SeqCst) {
		return;
	}

	// Register consensus metrics
	if let Err(e) = prometheus::register(Box::new(CONSENSUS_METRICS.block_height.clone())) {
		warn!("Failed to register consensus block height metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(CONSENSUS_METRICS.round.clone())) {
		warn!("Failed to register consensus round metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(CONSENSUS_METRICS.committed_txns.clone())) {
		warn!("Failed to register consensus committed transactions metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(CONSENSUS_METRICS.round_timing.clone())) {
		warn!("Failed to register consensus round timing metric: {}", e);
	}

	// Register DB metrics
	if let Err(e) = prometheus::register(Box::new(DB_METRICS.db_size.clone())) {
		warn!("Failed to register DB size metric: {}", e);
	}

	// Register mempool metrics
	if let Err(e) = prometheus::register(Box::new(MEMPOOL_METRICS.txn_count.clone())) {
		warn!("Failed to register mempool transaction count metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(MEMPOOL_METRICS.processing_time.clone())) {
		warn!("Failed to register mempool processing time metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(MEMPOOL_METRICS.broadcast_time.clone())) {
		warn!("Failed to register mempool broadcast time metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(MEMPOOL_METRICS.pending_txns.clone())) {
		warn!("Failed to register mempool pending transactions metric: {}", e);
	}

	// Register transaction metrics
	if let Err(e) = prometheus::register(Box::new(TRANSACTION_METRICS.retrieval_latency.clone())) {
		warn!("Failed to register transaction retrieval latency metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(TRANSACTION_METRICS.commit_latency.clone())) {
		warn!("Failed to register transaction commit latency metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(TRANSACTION_METRICS.save_latency.clone())) {
		warn!("Failed to register transaction save latency metric: {}", e);
	}

	// Register sync metrics
	if let Err(e) = prometheus::register(Box::new(SYNC_METRICS.synced_version.clone())) {
		warn!("Failed to register sync version metric: {}", e);
	}
	if let Err(e) = prometheus::register(Box::new(SYNC_METRICS.sync_progress.clone())) {
		warn!("Failed to register sync progress metric: {}", e);
	}

	// Mark metrics as initialized
	METRICS_INITIALIZED.store(true, Ordering::SeqCst);
}

/// Initialize and start the telemetry service
pub fn start_telemetry_service() {
	// Configure environment variables for telemetry
	std::env::set_var("APTOS_FORCE_ENABLE_TELEMETRY", "1");
	std::env::set_var("PROMETHEUS_METRICS_ENABLED", "1");
	std::env::set_var("APTOS_METRICS_PORT", "9464");
	std::env::set_var("APTOS_METRICS_HOST", "0.0.0.0");
	std::env::set_var("APTOS_DISABLE_TELEMETRY_PUSH_METRICS", "1");

	// Enable additional metrics collection
	std::env::set_var("APTOS_ENABLE_CONSENSUS_METRICS", "1");
	std::env::set_var("APTOS_ENABLE_DB_METRICS", "1");
	std::env::set_var("APTOS_ENABLE_MEMPOOL_METRICS", "1");
	std::env::set_var("APTOS_ENABLE_NETWORK_METRICS", "1");
	std::env::set_var("APTOS_ENABLE_STORAGE_METRICS", "1");
	std::env::set_var("APTOS_ENABLE_VM_METRICS", "1");

	// Configure OTEL if endpoint is provided
	if let Ok(otel_endpoint) = std::env::var("MOVEMENT_OTEL_ENDPOINT") {
		std::env::set_var("APTOS_OTEL_ENDPOINT", &otel_endpoint);
		info!("Configured OTEL endpoint: {}", otel_endpoint);
	}

	// Register custom metrics
	register_custom_metrics();

	let runtime = RUNTIME.clone();

	// Start the metrics server first
	let metrics_host =
		std::env::var("APTOS_METRICS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
	let metrics_port = std::env::var("APTOS_METRICS_PORT").unwrap_or_else(|_| "9464".to_string());
	let addr: SocketAddr = format!("{}:{}", metrics_host, metrics_port).parse().unwrap();

	// Create metrics handler that includes both aptos-core metrics and our custom metrics
	let metrics_handler = || {
		let encoder = prometheus::TextEncoder::new();
		let mut buffer = vec![];
		encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
		String::from_utf8(buffer).unwrap()
	};

	let metrics_route = warp::path!("metrics").map(move || {
		let metrics = metrics_handler();
		warp::reply::with_header(metrics, "content-type", "text/plain; version=0.0.4")
	});

	info!("Starting metrics server at http://{}", addr);

	// Spawn the metrics server
	runtime.spawn(async move {
		warp::serve(metrics_route).run(addr).await;
	});

	// Start the metrics collector
	let collector = Arc::new(AptosMetricsCollector::new());
	runtime.spawn(async move {
		let mut interval = time::interval(Duration::from_secs(1));
		loop {
			interval.tick().await;
			collector.collect_metrics();
		}
	});

	// Now start the telemetry service
	runtime.spawn(async move {
		let result = aptos_telemetry::service::start_telemetry_service(
			DEFAULT_NODE_CONFIG.clone(),
			*DEFAULT_CHAIN_ID,
			DEFAULT_BUILD_INFO.clone(),
			None,
			None,
		);

		match &result {
			Some(_) => info!("Telemetry service started successfully"),
			None => {
				warn!("Failed to start telemetry service. This might be expected if telemetry is disabled.");
				warn!("Metrics should still be available at the endpoint.");
			}
		}
	});
}

/// Ensures telemetry is initialized
pub fn ensure_telemetry_initialized() {
	start_telemetry_service();
}

/// Returns the metrics endpoint URL that Prometheus should scrape
pub fn get_metrics_endpoint() -> String {
	ensure_telemetry_initialized();
	"http://localhost:9464/metrics".to_string()
}

/// Shutdown the telemetry service
pub fn shutdown_telemetry() {
	info!("Shutting down telemetry service...");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_metrics_endpoint() {
		assert_eq!(get_metrics_endpoint(), "http://localhost:9464/metrics");
	}
}
