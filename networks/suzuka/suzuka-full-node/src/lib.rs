pub mod admin;
pub mod common_args;
pub mod node;
pub mod run;
#[cfg(test)]
pub mod tests;
use clap::Parser;

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum SuzukaFullNode {
	#[clap(subcommand)]
	Admin(admin::Admin),
}

impl SuzukaFullNode {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			SuzukaFullNode::Syncing(syncing) => syncing.execute().await,
		}
	}
}
