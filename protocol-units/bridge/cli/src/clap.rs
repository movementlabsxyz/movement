pub mod eth_to_movement;
use clap::{Parser, Subcommand};

use crate::eth_to_movement::EthSubCommands;

#[derive(Parser)]
#[command(name = "Movementlabs Bridge CLI")]
#[command(about = "Command line interface to perform an atomic bridge transfers", long_about = None)]
pub struct CliOptions {
	#[command(subcommand)]
	pub command: BridgeCommands,
}

#[derive(Subcommand)]
pub enum BridgeCommands {
	/// Start the Bridge Relayer Service
	/// Ethereum to Movement Labs bridge commands
	#[command(subcommand)]
	BridgeEthToMovETH(EthSubCommands),
}
