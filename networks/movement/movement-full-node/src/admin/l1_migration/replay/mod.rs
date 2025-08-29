use crate::admin::l1_migration::replay::da_height::DaHeight;
use crate::admin::l1_migration::replay::replay::DaReplayTransactions;
use clap::Parser;

mod compare;
mod da_height;
mod replay;
mod types;

#[derive(Parser, Debug)]
#[clap(name = "Movement Da-Sequencer replay tool", author, disable_version_flag = true)]
pub enum ValidationTool {
	Replay(DaReplayTransactions),
	ExtractDaHeight(DaHeight),
}

impl ValidationTool {
	pub async fn execute(&self) -> anyhow::Result<()> {
		match self {
			ValidationTool::Replay(cmd) => cmd.run().await,
			ValidationTool::ExtractDaHeight(cmd) => cmd.run(),
		}
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	ValidationTool::command().debug_assert()
}
