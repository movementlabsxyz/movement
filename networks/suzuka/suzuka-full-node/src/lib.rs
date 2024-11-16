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
	Run(run::Run),
}

impl SuzukaFullNode {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Self::Admin(admin) => admin.execute().await,
			Self::Run(run) => run.execute().await,
		}
	}
}
