use crate::common_args::MovementArgs;
use clap::Parser;
use movement_config::migrations::elsa_to_biarritz_rc1::MigrateElsaToBiarritzRc1;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Migrates the config from Elsa to Biarritz RC1")]
pub struct ElsaToBiarritzRc1 {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl ElsaToBiarritzRc1 {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let dot_movement = self.movement_args.dot_movement()?;
		let config = dot_movement.migrate_elsa_to_biarritz_rc1().await?;

		Ok(())
	}
}
