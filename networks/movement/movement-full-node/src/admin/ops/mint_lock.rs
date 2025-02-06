use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Mints and locks tokens.")]
pub struct MintLock {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl MintLock {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
