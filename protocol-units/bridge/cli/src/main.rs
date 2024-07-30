use anyhow::Result;
use bridge_cli::clap::{Cli, Commands};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
	inner_main().await
}

async fn inner_main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let cli = Cli::parse();

	match &cli.command {
		Commands::Eth(command) => {
			bridge_cli::eth::command(command).await?;
		}
	}

	Ok(())
}
