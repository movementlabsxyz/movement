use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
pub struct Delete {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Delete {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let dot_movement_dir = self.movement_args.dot_movement()?;

		dot_movement

		Ok(())
	}
}
