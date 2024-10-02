use bridge_cli::clap::{CliOptions, Commands};
use bridge_service::chains::{
	ethereum::client::{Config as EthConfig, EthClient},
	movement::client::{Config as MovementConfig, MovementClient},
};
use clap::Parser;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
	inner_main().await.map_err(|e| eyre::eyre!(e))
}

async fn inner_main() -> anyhow::Result<()> {
	tracing_subscriber::fmt::init();

	let eth_client = EthClient::new(EthConfig::build_for_test())
		.await
		.expect("Failed to create EthClient");
	let _movement_client = MovementClient::new(&MovementConfig::build_for_test())
		.await
		.expect("Failed to create MovementClient");

	let cli = CliOptions::parse();

	match &cli.command {
		Commands::BridgeEthToMovETH(command) => {
			bridge_cli::eth_to_movement::execute(command).await?;
		}
	}

	Ok(())
}
