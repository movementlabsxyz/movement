use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Bridge CLI")]
#[command(about = "Command line interface to perform an atomic bridge", long_about = None)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Ethereum bridge commands
	#[command(subcommand)]
	Eth(EthCommands),
}

#[derive(Subcommand)]
pub enum EthCommands {
	/// Initiate a bridge transfer
	Swap {
		#[arg(long)]
		recipient: String,
		#[arg(long)]
		amount: u64,
	},
}
