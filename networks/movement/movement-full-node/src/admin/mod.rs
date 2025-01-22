pub mod force_commitment;
pub mod rotate_key;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Admin {
	ForceCommitment(force_commitment::ForceCommitment),
	RotateKey(rotate_key::RotateKey),
}

impl Admin {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Admin::ForceCommitment(force_commitment) => force_commitment.execute().await,
			Admin::RotateKey(rotate_key) => rotate_key.execute().await,
		}
	}
}
