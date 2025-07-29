pub mod elsa_to_biarritz_rc1;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for migrating configs")]
pub enum Migrate {
	ElsaToBiarritzRc1(elsa_to_biarritz_rc1::ElsaToBiarritzRc1),
}

impl Migrate {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Migrate::ElsaToBiarritzRc1(cmd) => cmd.execute().await,
		}
	}
}
