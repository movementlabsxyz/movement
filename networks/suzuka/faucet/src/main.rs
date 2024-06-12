// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_faucet_core::server::{self, Server};
use aptos_logger::info;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    server: Server,
}

impl Args {
    pub async fn run_command(&self) -> Result<()> {
        self.server.run_command().await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut root_args = Args::parse();
    let modified_server = match root_args.server {
        Server::RunSimple(server) => {
            
        },
        server => server,
    };
  
    aptos_logger::Logger::builder()
        .level(aptos_logger::Level::Info)
        .build();

    info!("Running with root args: {:#?}", root_args);

    root_args.run_command().await
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
