#[allow(unused_imports)]
use anyhow::Context;
use crate::common_args::MovementArgs;
use aptos_sdk::{coin_client::CoinClient, rest_client::{Client, FaucetClient}, transaction_builder::TransactionBuilder, types::{chain_id::ChainId, transaction::EntryFunction, LocalAccount}};
use clap::Parser;
use once_cell::sync::Lazy;
use url::Url;
use std::{str::FromStr, time::{SystemTime, UNIX_EPOCH}};
use aptos_sdk::{
	coin_client::CoinClient,
	crypto::{SigningKey, ValidCryptoMaterialStringExt},
	move_types::{
		identifier::Identifier,
		language_storage::{ModuleId, TypeTag},
	},
	rest_client::{Client, FaucetClient, Transaction},
	transaction_builder::TransactionFactory,
	types::{account_address::AccountAddress, transaction::TransactionPayload},
};

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Mints and locks tokens.")]
pub struct Mint {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

// :!:>section_1c
static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let node_connection_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let node_connection_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_port
		.clone();

	let node_connection_url =
		format!("http://{}:{}", node_connection_address, node_connection_port);

	Url::from_str(node_connection_url.as_str()).unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
	let faucet_listen_address = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_hostname
		.clone();
	let faucet_listen_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_faucet_rest_connection_port
		.clone();

	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);

	Url::from_str(faucet_listen_url.as_str()).unwrap()
});

static MAPTOS_PRIVATE_KEY: Lazy<Url> = Lazy::new(|| {
	let faucet_listen_port = SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.chain
		.maptos_private_key
		.clone();

	let faucet_listen_url = format!("http://{}:{}", faucet_listen_address, faucet_listen_port);

	Url::from_str(faucet_listen_url.as_str()).unwrap()
});

const DEAD_ADDRESS: &str = "000000000000000000000000000000000000000000000000000000000000dead";

impl Mint {
	
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let rest_client = Client::new(NODE_URL.clone());
		let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
		let coin_client = CoinClient::new(&rest_client);
		let dead_address = AccountAddress::from_str(DEAD_ADDRESS)?;
		let chain_id = rest_client
			.get_index()
			.await
			.context("failed to get chain ID")?
			.inner()
			.chain_id;

			let mut core_resources_account: LocalAccount = LocalAccount::from_private_key(
				MAPTOS_PRIVATE_KEY.clone().as_str(),
				0,
			)?;

		tracing::info!("Created core resources account");
		tracing::debug!("core_resources_account address: {}", core_resources_account.address());

		// Create account for transactions and gas collection
		let private_key = SUZUKA_CONFIG
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key
			.to_string();

		let create_dead_transaction =
		core_resources_account.sign_with_transaction_builder(TransactionBuilder::new(
			TransactionPayload::EntryFunction(EntryFunction::new(
				ModuleId::new(
					AccountAddress::from_hex_literal("0x1")?,
					Identifier::new("aptos_account")?,
				),
				Identifier::new("create_account")?,
				vec![],
				vec![bcs::to_bytes(&dead_address)?],
			)),
			SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
			ChainId::new(chain_id),
		).sequence_number(core_resources_account.sequence_number()));

		Ok(())
	}
}
