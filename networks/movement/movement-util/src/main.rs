#![forbid(unsafe_code)]

use clap::*;
use movement_util::MovementOpts;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let movement_opts = MovementOpts::parse();

	movement_opts.execute().await?;

	Ok(())
}
