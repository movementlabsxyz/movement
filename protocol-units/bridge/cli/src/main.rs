use anyhow::Result;
use clap::{Parser, Subcommand};

mod eth;
mod movement;

#[derive(Parser)]
#[command(name = "bridge-cli")]
#[command(about = "A CLI for interacting with various bridge clients", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Eth {
		#[command(subcommand)]
		eth_command: eth::EthCommands,
	},

	Movement {
		#[command(subcommand)]
		movement_command: movement::MovementCommands,
	},
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	match &cli.command {
		Commands::Eth { eth_command } => {
			eth::run(eth_command).await?;
		}
		Commands::Movement { movement_command } => {
			movement::run(movement_command).await?;
		}
	}

	Ok(())
}
