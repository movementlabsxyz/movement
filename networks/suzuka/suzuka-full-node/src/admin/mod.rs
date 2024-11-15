pub mod force_commitment;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Admin {
	ForceCommitment(delete_resource::DeleteResource),
}

impl Admin {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Admin::Delete(delete) => delete.execute().await,
			/*Admin::Downsync(downsync) => downsync.execute(),
			Admin::Upsync(upsync) => upsync.execute(),*/
		}
	}
}
