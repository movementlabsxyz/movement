#![forbid(unsafe_code)]
use clap::*;
use movement_full_node::MovementFullNode;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Initialize aptos logger
	aptos_logger::Logger::builder().level(aptos_logger::Level::Info).build();

	// Initialize telemetry if MOVEMENT_METRICS_ADDR is set
	if std::env::var("MOVEMENT_METRICS_ADDR").is_ok() {
		movement_tracing::ensure_telemetry_initialized();
	}

	let suzuka_util = MovementFullNode::parse();
	let result = suzuka_util.execute().await;
	result
}
