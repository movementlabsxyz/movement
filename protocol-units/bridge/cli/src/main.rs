use bridge_cli::clap::{CliOptions, Commands};
use clap::Parser;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
	inner_main().await.map_err(|e| eyre::eyre!(e))
}

async fn inner_main() -> anyhow::Result<()> {
	tracing_subscriber::fmt::init();

	let cli = CliOptions::parse();

	match &cli.command {
		Commands::Bridge(command) => {
			bridge_cli::eth_to_moveth::execute(command).await?;
		}
	}

	Ok(())
}
