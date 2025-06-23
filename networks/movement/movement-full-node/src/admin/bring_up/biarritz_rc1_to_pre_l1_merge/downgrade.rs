use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Downgrades from Pre L1 Merge to Biarritz RC1")]
pub struct Downgrade {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Downgrade {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
