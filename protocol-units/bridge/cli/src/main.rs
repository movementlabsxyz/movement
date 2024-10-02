use anyhow::Result;
use bridge_service::chains::{
	ethereum::client::{Config as EthConfig, EthClient},
	movement::client::{Config as MovementConfig, MovementClient},
};
use clap::Parser;
use cli_client::Client;
use subcommands::{BridgeCommands, CliOptions};

mod cli_client;
mod subcommands;

#[tokio::main]
async fn main() -> Result<()> {
	inner_main().await.map_err(|e| anyhow::anyhow!(e))
}

async fn inner_main() -> anyhow::Result<()> {
	tracing_subscriber::fmt::init();

	let eth_client = EthClient::new(EthConfig::build_for_test())
		.await
		.expect("Failed to create EthClient");
	let movement_client = MovementClient::new(&MovementConfig::build_for_test())
		.await
		.expect("Failed to create MovementClient");

	let client = Client::new(eth_client, movement_client); // Pass by value, not reference

	let cli = CliOptions::parse();
	match &cli.command {
		BridgeCommands::EthToMovement(command) => {
			client.eth_movement_execute(command).await?;
		}
		BridgeCommands::MovementToEth(command) => {
			client.movement_eth_execute(command).await?;
		}
	}

	Ok(())
}
