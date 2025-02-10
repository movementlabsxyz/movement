pub mod elsa_to_biarritz_rc1;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for framework upgrades")]
pub enum BringUp {
	#[clap(subcommand)]
	ElsaToBiarritzRc1(elsa_to_biarritz_rc1::ElsaToBiarritzRc1),
}

impl BringUp {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			BringUp::ElsaToBiarritzRc1(elsa_to_biarritz_rc1) => {
				elsa_to_biarritz_rc1.execute().await
			}
		}
	}
}
