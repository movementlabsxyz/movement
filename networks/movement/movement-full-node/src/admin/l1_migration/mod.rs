use crate::admin::l1_migration::change_epoch_duration::ChangeEpochDuration;
use clap::Subcommand;

mod change_epoch_duration;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for rotating keys")]
pub enum L1Migration {
	ChangeEpoch(ChangeEpochDuration),
}

impl L1Migration {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			L1Migration::ChangeEpoch(change_epoch_duration) => {
				change_epoch_duration.execute().await
			}
		}
	}
}
