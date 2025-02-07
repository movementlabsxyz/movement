use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Rotates the key for an MCR validator.")]
pub struct McrValidator {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub height: Option<u64>,
}

impl McrValidator {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		unimplemented!()
	}
}
