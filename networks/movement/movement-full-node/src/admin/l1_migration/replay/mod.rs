use crate::admin::l1_migration::replay::da_height::DaHeight;
use crate::admin::l1_migration::replay::replay::DaReplayTransactions;
use clap::Parser;

mod da_height;
mod replay;
mod types;

#[derive(Parser, Debug)]
#[clap(name = "Movement Da-Sequencer replay tool", author, disable_version_flag = true)]
pub enum ApiReplayTool {
	Replay(DaReplayTransactions),
	ExtractDaHeight(DaHeight),
}

impl ApiReplayTool {
	pub async fn execute(&self) -> anyhow::Result<()> {
		match self {
			ApiReplayTool::Replay(cmd) => cmd.run().await,
			ApiReplayTool::ExtractDaHeight(cmd) => cmd.run(),
		}
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	ApiReplayTool::command().debug_assert()
}
