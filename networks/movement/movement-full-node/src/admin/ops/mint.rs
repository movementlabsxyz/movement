use buildtime_helpers::cargo::cargo_workspace;
use anyhow::Context;
use tokio::process::Command;
use crate::common_args::MovementArgs;
use aptos_sdk::{
	coin_client::CoinClient,	
	rest_client::{Client, FaucetClient}, types::{chain_id::ChainId, test_helpers::transaction_test_helpers, transaction::Script, LocalAccount}
};
use clap::Parser;
use once_cell::sync::Lazy;
use url::Url;
use std::{fs, path::PathBuf, str::FromStr, time::{SystemTime, UNIX_EPOCH}};
use aptos_sdk::types::transaction::TransactionPayload;

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

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Mints and locks tokens.")]
pub struct Mint {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
}

impl Mint {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let rest_client = Client::new(NODE_URL.clone());
		let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
		let coin_client = CoinClient::new(&rest_client);
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

		let core_resources_account: LocalAccount =
			LocalAccount::from_private_key(&private_key.clone(), 0)?;

		tracing::debug!("coreresources_account address: {}", core_resources_account.address());

		let _ = Command::new("movement")
			.args(["move", "compile", "--package-dir", "protocol-units/bridge/move-modules"])
			.status()
			.await?;
	
		let root: PathBuf= cargo_workspace()?;
		let additional_path =
			"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/";
		let combined_path = root.join(additional_path);
	
		let enable_bridge_code = fs::read(combined_path.join("enable_bridge_feature.mv"))?;
		let enable_bridge_script_payload =
			TransactionPayload::Script(Script::new(enable_bridge_code, vec![], vec![]));
	
		let enable_bridge_script_transaction =
			transaction_test_helpers::get_test_signed_transaction_with_chain_id(
				core_resources_account.address(),
				core_resources_account.sequence_number(),
				&core_resources_account.private_key(),
				core_resources_account.public_key().clone(),
				Some(enable_bridge_script_payload),
				SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
				100,
				None,
				ChainId::new(chain_id),
			);
	
		rest_client
			.submit_and_wait(&enable_bridge_script_transaction)
			.await
			.context("Failed to enable bridge script transaction")?;
	
		core_resources_account.increment_sequence_number();
	
		let store_mint_burn_caps_code = fs::read(combined_path.join("store_mint_burn_caps.mv"))?;
		let store_mint_burn_caps_script_payload =
			TransactionPayload::Script(Script::new(store_mint_burn_caps_code, vec![], vec![]));
	
		let store_mint_burn_caps_script_transaction =
			transaction_test_helpers::get_test_signed_transaction_with_chain_id(
				core_resources_account.address(),
				core_resources_account.sequence_number(),
				&core_resources_account.private_key(),
				core_resources_account.public_key().clone(),
				Some(store_mint_burn_caps_script_payload),
				SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
				100,
				None,
				ChainId::new(chain_id),
			);
	
		rest_client
			.submit_and_wait(&store_mint_burn_caps_script_transaction)
			.await
			.context("Failed to store_mint_burn_caps script transaction")?;
		core_resources_account.increment_sequence_number();
	
		tracing::info!("Bridge feature enabled and mint burn caps stored");			

		Ok(())
	}
}
