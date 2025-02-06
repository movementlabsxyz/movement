pub mod downgrade;
pub mod upgrade;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for migrating from Elsa to Biarritz RC1")]
pub enum ElsaToBiarritzRc1 {
	Upgrade(upgrade::Upgrade),
	Downgrade(downgrade::Downgrade),
}

impl ElsaToBiarritzRc1 {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			ElsaToBiarritzRc1::Upgrade(upgrade) => upgrade.execute().await,
			ElsaToBiarritzRc1::Downgrade(downgrade) => downgrade.execute().await,
		}
	}
}
