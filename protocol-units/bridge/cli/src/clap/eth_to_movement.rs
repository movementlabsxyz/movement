use alloy::signers::local::PrivateKeySigner;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct SharedArgs {
	#[arg(long)]
	pub eth_private_key: PrivateKeySigner,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Initiate a bridge transfer
	Swap {
		#[command(flatten)]
		args: SharedArgs,
		#[arg(long)]
		recipient: String,
		#[arg(long)]
		amount: u64,
	},
	/// Resume a bridge transfer
	Resume {
		#[command(flatten)]
		args: SharedArgs,
		#[arg(long)]
		transfer_id: String,
	},
}
