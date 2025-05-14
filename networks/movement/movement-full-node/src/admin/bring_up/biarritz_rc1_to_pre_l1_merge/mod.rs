pub mod downgrade;
pub mod upgrade;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(
	rename_all = "kebab-case",
	about = "Commands for migrating from Biarritz RC1 to Pre-L1 Merge"
)]
pub enum BiarritzRc1ToPreL1Merge {
	Upgrade(upgrade::Upgrade),
	Downgrade(downgrade::Downgrade),
}

impl BiarritzRc1ToPreL1Merge {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			BiarritzRc1ToPreL1Merge::Upgrade(upgrade) => upgrade.execute().await,
			BiarritzRc1ToPreL1Merge::Downgrade(downgrade) => downgrade.execute().await,
		}
	}
}
