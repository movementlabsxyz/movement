#![forbid(unsafe_code)]
use clap::*;
use movement_full_node::MovementFullNode;
use tracing_subscriber::EnvFilter;
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	// Initialize default tracing
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();
	let suzuka_util = MovementFullNode::parse();
	let result = suzuka_util.execute().await;
	result
}
