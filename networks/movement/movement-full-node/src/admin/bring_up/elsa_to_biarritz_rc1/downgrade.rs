use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Downgrades from Biarritz RC1 to Elsa")]
pub struct Downgrade {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Downgrade {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
