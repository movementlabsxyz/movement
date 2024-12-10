pub mod force_commitment;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Admin {
	ForceCommitment(force_commitment::ForceCommitment),
}

impl Admin {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Admin::ForceCommitment(force_commitment) => force_commitment.execute().await,
		}
	}
}
