use crate::common_args::MovementArgs;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Rotates the key for a known signer.")]
pub struct KnownSigner {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub height: Option<u64>,
}

impl KnownSigner {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		unimplemented!()
	}
}
