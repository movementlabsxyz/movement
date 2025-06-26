pub mod common_args;
pub mod syncing;
use clap::Subcommand;

#[derive(Subcommand)]
#[clap(rename_all = "kebab-case")]
pub enum Util {
	#[clap(subcommand)]
	Syncing(syncing::Syncing),
}

impl Util {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Util::Syncing(syncing) => syncing.execute().await,
		}
	}
}
