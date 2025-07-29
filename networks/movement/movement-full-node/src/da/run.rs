use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Runs Da Sequencer.")]
pub struct DaRun {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl DaRun {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		// get the config file
		let dot_movement = self.movement_args.dot_movement()?;
		movement_da_sequencer_node::start(dot_movement).await
	}
}
