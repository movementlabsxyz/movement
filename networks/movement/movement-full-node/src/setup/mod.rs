pub mod all;
pub mod da;
pub mod full_node;
pub mod replicat;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Setup {
	All(all::All),
	FullNode(full_node::FullNode),
	Da(da::Da),
	Replicat(replicat::Replicat),
}

impl Setup {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Setup::All(all) => all.execute().await,
			Setup::FullNode(full_node) => full_node.execute().await,
			Setup::Da(da) => da.execute().await,
			Setup::Replicat(replicat) => replicat.execute().await,
		}
	}
}
