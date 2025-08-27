use crate::admin::l1_migration::replay::ApiReplayTool;
use clap::Subcommand;

mod replay;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for rotating keys")]
pub enum L1Migration {
	#[clap(subcommand)]
	Replay(ApiReplayTool),
}

impl L1Migration {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			L1Migration::Replay(tool) => tool.execute().await,
		}
	}
}
