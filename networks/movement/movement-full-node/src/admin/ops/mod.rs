pub mod burn;
pub mod mint;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for bespoke network operations")]
pub enum Ops {
	Mint(mint::Mint),
	//Burn(burn::Burn),
}

impl Ops {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Ops::Mint(mint) => mint.execute().await,
			//Ops::Burn(burn) => burn.execute().await,
		}
	}
}
