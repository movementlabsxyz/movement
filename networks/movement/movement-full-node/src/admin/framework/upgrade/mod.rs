pub mod biarritz_rc1;
pub mod commit_hash;
pub mod elsa;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for framework upgrades")]
pub enum Upgrade {
	BiarritzRc1(biarritz_rc1::BiarritzRc1),
	CommitHash(commit_hash::CommitHash),
	Elsa(elsa::Elsa),
}

impl Upgrade {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Upgrade::BiarritzRc1(biarritz_rc1) => biarritz_rc1.execute().await,
			Upgrade::CommitHash(commit_hash) => commit_hash.execute().await,
			Upgrade::Elsa(elsa) => elsa.execute().await,
		}
	}
}
