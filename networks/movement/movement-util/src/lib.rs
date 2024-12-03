pub mod common_args;
pub mod syncing;
use clap::Parser;

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum MovementOpts {
	#[clap(subcommand)]
	Syncing(syncing::Syncing),
}

impl MovementOpts {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			MovementOpts::Syncing(syncing) => syncing.execute().await,
		}
	}
}
