pub mod fund;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for bespoke network operations")]
pub enum GovernedGasPool {
	Fund(fund::Fund),
}

impl GovernedGasPool {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			GovernedGasPool::Fund(fund) => fund.execute().await,
		}
	}
}
