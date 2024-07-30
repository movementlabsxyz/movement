use clap::{Args, Parser, Subcommand};

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

#[derive(Args)]
pub struct EthShared {
	#[arg(long)]
	pub private_key: String,
}

#[derive(Subcommand)]
pub enum EthCommands {
	/// Initiate a bridge transfer
	Swap {
		#[command(flatten)]
		args: EthShared,
		#[arg(long)]
		recipient: String,
		#[arg(long)]
		amount: u64,
	},
	/// Resume a bridge transfer
	Resume {
		#[command(flatten)]
		args: EthShared,
		#[arg(long)]
		transfer_id: String,
	},
}
