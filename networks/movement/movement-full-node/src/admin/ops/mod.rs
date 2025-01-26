pub mod mint_lock;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for bespoke network operations")]
pub enum Ops {
	MintLock(mint_lock::MintLock),
}

impl Ops {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Ops::MintLock(mint_lock) => mint_lock.execute().await,
		}
	}
}
