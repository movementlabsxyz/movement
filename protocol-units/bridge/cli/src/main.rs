use anyhow::Result;
use bridge_cli::clap::{CliOptions, Commands};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
	inner_main().await
}

async fn inner_main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let cli = CliOptions::parse();

	match &cli.command {
		Commands::BridgeEthToMovETH(command) => {
			bridge_cli::eth_to_moveth::execute(command).await?;
		}
	}

	Ok(())
}
