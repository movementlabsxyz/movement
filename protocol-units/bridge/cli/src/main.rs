use anyhow::Result;
use clap::Parser;

use bridge_cli::clap::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
	inner_main().await
}

async fn inner_main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let cli = Cli::parse();

	// Load configuration
	let config = Config::default();

	match &cli.command {
		Commands::Eth(command) => {
			bridge_cli::eth::command(command).await?;
		}
	}

	Ok(())
}
