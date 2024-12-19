#![forbid(unsafe_code)]

use clap::*;
use movement_full_node::MovementFullNode;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let tracing_config = movement_tracing::Config::from_env()?;
	let _guard = movement_tracing::init_tracing_subscriber(
		env!("CARGO_BIN_NAME"),
		env!("CARGO_PKG_VERSION"),
		&tracing_config,
	);

	let suzuka_util = MovementFullNode::parse();

	suzuka_util.execute().await?;

	Ok(())
}
