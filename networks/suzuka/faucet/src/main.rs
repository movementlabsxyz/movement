// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_config::keys::ConfigKey;
use aptos_faucet_core::{
	funder::ApiConnectionConfig,
	server::{self, Server},
};
use tracing::info;
use aptos_sdk::crypto::{ed25519::Ed25519PrivateKey, ValidCryptoMaterialStringExt};
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
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;

	// get the connection url
	let connection_host =
		config.execution_config.maptos_config.faucet.maptos_rest_connection_hostname;
	let connection_port = config.execution_config.maptos_config.faucet.maptos_rest_connection_port;
	let connection_url = format!("http://{}:{}", connection_host, connection_port);

	// get the key
	let private_key = config.execution_config.maptos_config.chain.maptos_private_key.clone();

	// get the chain id
	let chain_id = config.execution_config.maptos_config.chain.maptos_chain_id.clone();

	// get the listener host and port
	let listener_host =
		config.execution_config.maptos_config.faucet.maptos_faucet_rest_listen_hostname;
	let listener_port = config.execution_config.maptos_config.faucet.maptos_faucet_rest_listen_port;

	let mut root_args = Args::parse();
	let modified_server = match root_args.clone().server {
		Server::RunSimple(mut server) => {
			server.api_connection_config = ApiConnectionConfig::new(
				connection_url.parse()?,
				/// The config will use an encoded key if one is provided
				"/not/a/real/path".to_string().into(),
				Some(ConfigKey::new(private_key)),
				chain_id,
			);
			server.listen_address = listener_host.to_string();
			server.listen_port = listener_port;
			Server::RunSimple(server)
		}
		server => server,
	};
	root_args.server = modified_server;
	info!("Running with root args: {:#?}", root_args);

	root_args.run_command().await
}

#[test]
fn verify_tool() {
	use clap::CommandFactory;
	Args::command().debug_assert()
}
