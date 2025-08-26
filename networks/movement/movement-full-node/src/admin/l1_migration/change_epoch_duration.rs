use crate::common_args::MovementArgs;
use clap::Parser;
use mvt_aptos_l1_migration::set_epoch_duration;

const TWO_HOURS_EPOCH_DURATION: u64 = 7_200_000_000; // default 2hours, 7_200_000_000 micro second.

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Rotates the key for a core resource account.")]
pub struct ChangeEpochDuration {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub new_epoch_duration: Option<u64>,
}

impl ChangeEpochDuration {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		// get the movement config from dot movement
		let _dot_movement = self.movement_args.dot_movement()?;
		let epoch_duration = self.new_epoch_duration.unwrap_or(TWO_HOURS_EPOCH_DURATION);
		set_epoch_duration(epoch_duration).await?;
		Ok(())
	}
}
