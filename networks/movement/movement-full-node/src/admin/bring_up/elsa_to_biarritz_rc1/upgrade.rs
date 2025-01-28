use crate::common_args::MovementArgs;
use aptos_framework_elsa_to_biarritz_rc1_migration::MigrateElsaToBiarritzRc1;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Upgrades from Elsa to Biarritz RC1")]
pub struct Upgrade {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Upgrade {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		// get the movement config from dot movement
		let dot_movement = self.movement_args.dot_movement()?;

		// run the framework migration
		dot_movement.migrate_framework_from_elsa_to_biarritz_rc1().await?;

		Ok(())
	}
}
