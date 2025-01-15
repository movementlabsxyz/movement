pub mod server;
pub mod rotate_key;

use clap::Parser;

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum HsmDemo {
        #[clap(subcommand)]
        Server(server::Server),

        RotateKey(rotate_key::RotateKey),
}

impl HsmDemo {
        pub async fn run(&self) -> Result<(), anyhow::Error> {
                match self {
                        HsmDemo::Server(server) => server.run().await,
                        HsmDemo::RotateKey(cmd) => cmd.run().await,
                }
        }
}
