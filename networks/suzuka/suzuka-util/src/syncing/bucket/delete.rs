use crate::common_args::MovementArgs;
use clap::Parser;
use syncup::SyncupOperations;

#[derive(Debug, Parser, Clone)]
pub struct Delete {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Delete {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let config = self.movement_args.config().await?;
		config.remove_syncup_resources().await?;

		Ok(())
	}
}
