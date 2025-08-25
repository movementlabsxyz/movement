use crate::replay::DaReplayTransactions;
use clap::Parser;

mod replay;
mod types;

#[derive(Parser)]
#[clap(name = "Movement Da-Sequencer replay tool", author, disable_version_flag = true)]
pub enum ApiReplayTool {
	Replay(DaReplayTransactions),
	// Validate,
}

impl ApiReplayTool {
	pub async fn run(self) -> anyhow::Result<()> {
		match self {
			ApiReplayTool::Replay(cmd) => cmd.run().await,
			// ApiReplayTool::Validate => Err(anyhow::anyhow!("Validation tool is unimplemented")),
		}
	}
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	ApiReplayTool::command().debug_assert()
}
