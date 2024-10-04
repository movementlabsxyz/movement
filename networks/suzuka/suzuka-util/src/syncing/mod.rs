pub mod delete;
pub mod downsync;
pub mod upsync;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Control bucket-based syncing")]
pub enum Syncing {
	Delete(delete::Delete),
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
