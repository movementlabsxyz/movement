pub mod accepted_commitment;
pub mod commitment;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Settlement {
	Commitment(commitment::Commitment),
	AcceptedCommitment(accepted_commitment::AcceptedCommitment),
}

impl Settlement {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Settlement::Commitment(commitment) => commitment.execute().await,
			Settlement::AcceptedCommitment(accepted_commitment) => {
				accepted_commitment.execute().await
			}
		}
	}
}
