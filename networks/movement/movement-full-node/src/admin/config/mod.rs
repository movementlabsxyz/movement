pub mod migrate;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for managing configs")]
pub enum Config {
	#[clap(subcommand)]
	Migrate(migrate::Migrate),
}

impl Config {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Config::Migrate(migrate) => migrate.execute().await,
		}
	}
}
