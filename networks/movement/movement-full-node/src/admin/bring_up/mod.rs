pub mod biarritz_rc1_to_pre_l1_merge;
pub mod elsa_to_biarritz_rc1;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for framework upgrades")]
pub enum BringUp {
	#[clap(subcommand)]
	ElsaToBiarritzRc1(elsa_to_biarritz_rc1::ElsaToBiarritzRc1),
	#[clap(subcommand)]
	BiarritzRc1ToPreL1Merge(biarritz_rc1_to_pre_l1_merge::BiarritzRc1ToPreL1Merge),
}

impl BringUp {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			BringUp::ElsaToBiarritzRc1(elsa_to_biarritz_rc1) => {
				elsa_to_biarritz_rc1.execute().await
			}
			BringUp::BiarritzRc1ToPreL1Merge(biarritz_rc1_to_pre_l1_merge) => {
				biarritz_rc1_to_pre_l1_merge.execute().await
			}
		}
	}
}
