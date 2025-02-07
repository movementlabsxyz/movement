use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Upgrades the framework to Elsa.")]
pub struct Elsa {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Elsa {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
