pub mod force_commitment;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for administrating MCR from the node.")]
pub enum Mcr {
	ForceCommitment(force_commitment::ForceCommitment),
}

impl Mcr {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Mcr::ForceCommitment(force_commitment) => force_commitment.execute().await,
		}
	}
}
