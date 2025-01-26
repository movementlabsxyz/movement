use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Upgrades the framework to Biarritz RC1.")]
pub struct BiarritzRc1 {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl BiarritzRc1 {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
