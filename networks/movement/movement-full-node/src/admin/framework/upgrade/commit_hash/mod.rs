use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Upgrades the framework to a provided commit hash.")]
pub struct CommitHash {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl CommitHash {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
