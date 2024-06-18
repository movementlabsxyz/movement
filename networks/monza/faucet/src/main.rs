// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_faucet_core::{
    server::{self, Server},
    funder::ApiConnectionConfig
};
use aptos_logger::info;
use clap::Parser;
use aptos_config::keys::ConfigKey;
use aptos_sdk::crypto::{
    ed25519::Ed25519PrivateKey,
    ValidCryptoMaterialStringExt
};

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

    let dot_movement = dot_movement::DotMovement::try_from_env()?;
    let path = dot_movement.path().join("config.toml");
    let config = suzuka_config::Config::try_from_toml_file(&path)?;

    let listener_url = config.execution_config.try_aptos_config()?.try_aptos_faucet_listen_url()?;
    let listener_split = listener_url.split(":").collect::<Vec<&str>>();
    let listener_host = listener_split[0];
    let listener_port = listener_split[1].parse::<u16>()?;

    let mut root_args = Args::parse();
    let modified_server = match root_args.clone().server {
        Server::RunSimple(mut server) => {
            server.api_connection_config = ApiConnectionConfig::new(
                format!(
                    "http://{}", 
                    config.execution_config.try_aptos_config()?.try_aptos_rest_listen_url()?,
                ).parse()?,
                /// The config will use an encoded key if one is provided
                "/not/a/real/path".to_string().into(),
                Some(
                    ConfigKey::new(
                        config.execution_config.try_aptos_config()?.try_aptos_private_key()?
                    )
                ),
                config.execution_config.try_aptos_config()?.try_chain_id()?,
            );
            server.listen_address = listener_host.to_string();
            server.listen_port = listener_port;
            Server::RunSimple(server)
        },
        server => server,
    };
    root_args.server = modified_server;
  
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
