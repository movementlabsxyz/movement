#[allow(unused_imports)]
use anyhow::Context;
use tokio::process::Command;
use crate::common_args::MovementArgs;
use aptos_sdk::{coin_client::CoinClient, crypto::ed25519::{PrivateKey, Ed25519PrivateKey}, move_types::language_storage::StructTag, rest_client::{Client, FaucetClient}, transaction_builder::TransactionBuilder, types::{chain_id::ChainId, transaction::{EntryFunction, Script, TransactionArgument}, LocalAccount}};
use clap::Parser;
use once_cell::sync::Lazy;
use url::Url;
use std::{fs, str::FromStr, time::{SystemTime, UNIX_EPOCH}};
use aptos_sdk::{
	move_types::{
		identifier::Identifier,
		language_storage::{ModuleId, TypeTag},
	},
	
	types::{account_address::AccountAddress, transaction::TransactionPayload},
};



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

static MAPTOS_PRIVATE_KEY: Lazy<Ed25519PrivateKey> = Lazy::new(|| {
	let pk= SUZUKA_CONFIG
		.execution_config
		.maptos_config
		.chain
		.maptos_private_key
		.clone();

		pk
});

const DEAD_ADDRESS: &str = "000000000000000000000000000000000000000000000000000000000000dead";

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Burns tokens 🔥.")]
pub struct Burn {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Burn {
	
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let rest_client = Client::new(NODE_URL.clone());
		let dead_address = AccountAddress::from_str(DEAD_ADDRESS)?;
		let chain_id = rest_client
			.get_index()
			.await
			.context("failed to get chain ID")?
			.inner()
			.chain_id;


		let private_key = SUZUKA_CONFIG
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key
			.to_string();

		let core_resources_account: LocalAccount = LocalAccount::from_private_key(
			&private_key.clone(),
			0,
		)?;

		tracing::info!("Created core resources account");
		tracing::debug!("core_resources_account address: {}", core_resources_account.address());

		// Create account for transactions and gas collection
		
			// I know that we shouldn't compile on cmd execution, but we can optimise this later.
		let _compile_status = Command::new("movement")
			.args([
				"move",
				"compile",
				"--package-dir",
				"networks/movement/movement-full-node/ops/move-modules",
			])
			.status()
			.await
			.expect("Failed to execute `movement compile` command");

		let code = fs::read("networks/movement/movement-full-node/ops/move-modules/burn_from.move")?;

		let args = vec![TransactionArgument::Address(dead_address), TransactionArgument::U64(1), TransactionArgument::U8Vector(StructTag {
			address: AccountAddress::from_hex_literal("0x1")?,
			module: Identifier::new("coin")?,
			name: Identifier::new("BurnCapability")?,
			type_args: vec![StructTag{
				address: AccountAddress::from_hex_literal("0x1")?,
				module: Identifier::new("aptos_coin")?,
				name: Identifier::new("AptosCoin")?,
				type_args: vec![],
			}.into()],
		}.access_vector())];

		let script_payload = TransactionPayload::Script(Script::new(code, vec![], args));
	
		let tx_response = rest_client.submit_and_wait(&core_resources_account.sign_with_transaction_builder(
			TransactionBuilder::new(
				script_payload,
				SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
				ChainId::new(chain_id),
			).sequence_number(core_resources_account.sequence_number())
		)).await?;

		tracing::info!("Transaction submitted: {:?}", tx_response);
		
		Ok(())
	}
}
