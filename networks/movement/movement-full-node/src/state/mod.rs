pub mod node;
pub mod settlement;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum State {
	#[clap(subcommand)]
	Node(node::Node),
	#[clap(subcommand)]
	Settlement(settlement::Settlement),
}

impl State {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			State::Node(node) => node.execute().await,
			State::Settlement(settlement) => settlement.execute().await,
		}
	}
}
