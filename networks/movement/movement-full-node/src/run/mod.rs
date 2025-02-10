use crate::common_args::MovementArgs;
use crate::node::manager::Manager;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Runs the Suzuka Full Node")]
pub struct Run {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Run {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		// get the config file
		let dot_movement = self.movement_args.dot_movement()?;
		let config_file = dot_movement.try_get_or_create_config_file().await?;

		let manager = Manager::new(config_file).await?;
		manager.try_run().await?;

		Ok(())
	}
}
