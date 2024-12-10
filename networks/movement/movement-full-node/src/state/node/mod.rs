pub mod commitment;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Node {
	Commitment(commitment::Commitment),
}

impl Node {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Node::Commitment(commitment) => commitment.execute().await,
		}
	}
}
