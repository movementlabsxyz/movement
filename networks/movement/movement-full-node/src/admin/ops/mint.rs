use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Mints and locks tokens.")]
pub struct Mint {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Mints tokens 🌿.")]
impl Mint {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		
	}
}
