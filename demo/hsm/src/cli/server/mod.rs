pub mod ed25519;
pub mod secp256k1;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for signing")]
pub enum Server {
	#[clap(subcommand)]
	Ed25519(ed25519::Ed25519),
	#[clap(subcommand)]
	Secp256k1(secp256k1::Secp256k1),
}

impl Server {
	pub async fn run(&self) -> Result<(), anyhow::Error> {
		match self {
			Server::Ed25519(ed) => ed.run().await,
			Server::Secp256k1(sk) => sk.run().await,
		}
	}
}
