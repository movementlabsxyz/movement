use crate::common_args::MovementArgs;
use clap::Parser;
use syncup::SyncupOperations;

#[derive(Debug, Parser, Clone)]
#[clap(
	rename_all = "kebab-case",
	about = "Deletes the resource used for syncing across syncer ids"
)]
pub struct DeleteResource {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl DeleteResource {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let config = self.movement_args.config().await?;
		config.remove_syncup_resources().await?;

		Ok(())
	}
}
