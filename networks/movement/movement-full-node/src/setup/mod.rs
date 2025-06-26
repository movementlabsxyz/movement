pub mod all;
pub mod da;
pub mod full_node;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Setup {
	All(all::All),
	FullNode(full_node::FullNode),
	Da(da::Da),
}

impl Setup {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Setup::All(all) => all.execute().await,
			Setup::FullNode(full_node) => full_node.execute().await,
			Setup::Da(da) => da.execute().await,
		}
	}
}
