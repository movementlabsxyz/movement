pub mod upgrade;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for framework upgrades")]
pub enum Framework {
	#[clap(subcommand)]
	Upgrade(upgrade::Upgrade),
}

impl Framework {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Framework::Upgrade(upgrade) => upgrade.execute().await,
		}
	}
}
