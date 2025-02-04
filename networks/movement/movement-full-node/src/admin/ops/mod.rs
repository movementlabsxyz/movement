pub mod burn;
pub mod mint_to;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for bespoke network operations")]
pub enum Ops {
	MintTo(mint_to::MintTo),
	//Burn(burn::Burn),
}

impl Ops {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Ops::MintTo(mint_to) => mint_to.execute().await,
			//Ops::Burn(burn) => burn.execute().await,
		}
	}
}
