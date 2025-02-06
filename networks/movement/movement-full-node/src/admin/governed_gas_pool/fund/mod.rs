use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Funds from Biarritz RC1 to Elsa")]
pub struct Fund {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Fund {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		Ok(())
	}
}
