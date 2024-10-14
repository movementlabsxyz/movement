pub mod delete_resource;
pub mod downsync;
pub mod upsync;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Syncing {
	Delete(delete_resource::DeleteResource),
	/*Downsync(downsync::Downsync),
	Upsync(upsync::Upsync),*/
}

impl Syncing {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Syncing::Delete(delete) => delete.execute().await,
			/*Syncing::Downsync(downsync) => downsync.execute(),
			Syncing::Upsync(upsync) => upsync.execute(),*/
		}
	}
}
