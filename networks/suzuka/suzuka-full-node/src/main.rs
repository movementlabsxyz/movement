#![forbid(unsafe_code)]

use clap::*;
use suzuka_full_node::SuzukaFullNode;
const TIMING_LOG_ENV: &str = "SUZUKA_TIMING_LOG";
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let tracing_config =
		movement_tracing::Config { timing_log_path: env::var_os(TIMING_LOG_ENV).map(Into::into) };
	let _guard = movement_tracing::init_tracing_subscriber(tracing_config);

	let suzuka_util = SuzukaFullNode::parse();

	suzuka_util.execute().await?;

	Ok(())
}
