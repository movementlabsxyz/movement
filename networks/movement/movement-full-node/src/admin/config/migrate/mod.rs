pub mod elsa_to_biarritz_rc1;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for migrating configs")]
pub enum Migrate {
	Config(elsa_to_biarritz_rc1::ElsaToBiarritzRc1),
}

impl Migrate {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Migrate::Config(config) => config.execute().await,
		}
	}
}
