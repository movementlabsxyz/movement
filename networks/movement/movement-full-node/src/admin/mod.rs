pub mod bring_up;
pub mod framework;
pub mod governed_gas_pool;
pub mod mcr;
pub mod ops;
pub mod rotate_key;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Admin {
	#[clap(subcommand)]
	Mcr(mcr::Mcr),
	#[clap(subcommand)]
	RotateKey(rotate_key::RotateKey),
	#[clap(subcommand)]
	BringUp(bring_up::BringUp),
	#[clap(subcommand)]
	GovernedGasPool(governed_gas_pool::GovernedGasPool),
	#[clap(subcommand)]
	Ops(ops::Ops),
	#[clap(subcommand)]
	Framework(framework::Framework),
}

impl Admin {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Admin::Mcr(mcr) => mcr.execute().await,
			Admin::RotateKey(rotate_key) => rotate_key.execute().await,
			Admin::BringUp(bring_up) => bring_up.execute().await,
			Admin::GovernedGasPool(governed_gas_pool) => governed_gas_pool.execute().await,
			Admin::Ops(ops) => ops.execute().await,
			Admin::Framework(framework) => framework.execute().await,
		}
	}
}
