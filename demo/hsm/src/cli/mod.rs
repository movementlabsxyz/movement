pub mod server;
use clap::Parser;

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum HsmDemo {
	#[clap(subcommand)]
	Server(server::Server),
}

impl HsmDemo {
	pub async fn run(&self) -> Result<(), anyhow::Error> {
		match self {
			HsmDemo::Server(server) => server.run().await,
		}
	}
}
