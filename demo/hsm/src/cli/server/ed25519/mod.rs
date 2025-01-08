pub mod hashi_corp_vault;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for signing with Ed25519")]
pub enum Ed25519 {
	HashiCorpVault(hashi_corp_vault::HashiCorpVault),
}

impl Ed25519 {
	pub async fn run(&self) -> Result<(), anyhow::Error> {
		match self {
			Ed25519::HashiCorpVault(hcv) => hcv.run().await,
		}
	}
}
