pub mod bring_up;
pub mod framework;
pub mod mcr;
pub mod ops;
pub mod rotate_key;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for syncing")]
pub enum Admin {
	#[clap(subcommand)]
	Mcr(mcr::Mcr),
	#[clap(subcommand)]
	RotateKey(rotate_key::RotateKey),
}

impl Admin {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Admin::Mcr(mcr) => mcr.execute().await,
			Admin::RotateKey(rotate_key) => rotate_key.execute().await,
		}
	}
}
