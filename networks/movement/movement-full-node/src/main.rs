#![forbid(unsafe_code)]
use clap::*;
use movement_full_node::MovementFullNode;
// use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Initialize logger (for debugging)
	aptos_logger::Logger::builder().level(aptos_logger::Level::Debug).build();

	// Initialize default tracing
	// tracing_subscriber::fmt()
	// 	.with_env_filter(
	// 		EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
	// 	)
	// 	.init();

	// // Initialize telemetry if MOVEMENT_METRICS_ADDR is set
	// if std::env::var("MOVEMENT_METRICS_ADDR").is_ok() {
	// 	movement_tracing::ensure_telemetry_initialized();
	// }

	let suzuka_util = MovementFullNode::parse();
	let result = suzuka_util.execute().await;
	result
}
