pub mod eth_to_movement;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Movementlabs Bridge CLI")]
#[command(about = "Command line interface to perform an atomic bridge transfers", long_about = None)]
pub struct CliOptions {
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Ethereum to Movement Labs bridge commands
	#[command(subcommand)]
	Bridge(eth_to_movement::Commands),
}
