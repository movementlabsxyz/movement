use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Upgrades from Elsa to Biarritz RC1")]
pub struct Upgrade {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Upgrade {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
