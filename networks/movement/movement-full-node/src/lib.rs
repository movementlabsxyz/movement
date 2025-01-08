pub mod admin;
pub mod common_args;
pub mod da;
pub mod node;
pub mod run;
pub mod state;

#[cfg(test)]
pub mod tests;
use clap::Parser;

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum MovementFullNode {
	#[clap(subcommand)]
	Admin(admin::Admin),
	Run(run::Run),
	#[clap(subcommand)]
	State(state::State),
	#[clap(subcommand)]
	Da(da::Da),
}

impl MovementFullNode {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Self::Admin(admin) => admin.execute().await,
			Self::Run(run) => run.execute().await,
			Self::State(state) => state.execute().await,
			Self::Da(da) => da.execute().await,
		}
	}
}
