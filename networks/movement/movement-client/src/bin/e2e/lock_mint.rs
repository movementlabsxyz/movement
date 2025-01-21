#![allow(unused_imports)]
use anyhow::{Chain, Context};
use movement_client::crypto::ValidCryptoMaterialStringExt;

use aptos_sdk::{move_types::{
	identifier::Identifier, language_storage::{ModuleId, StructTag, TypeTag},
}, rest_client::Account};
use aptos_sdk::types::{
	account_address::AccountAddress, chain_id::ChainId, transaction::{EntryFunction, TransactionArgument, Script}, LocalAccount, AccountKey
};
use aptos_sdk::{
	rest_client::{
		aptos_api_types::{
			Address, EntryFunctionId, IdentifierWrapper, MoveModule, MoveModuleId, MoveStructTag,
			MoveType, ViewRequest,
		},
		Response,
	},
	transaction_builder::TransactionBuilder,
};
use aptos_types::{transaction::TransactionPayload, account_config::aptos_test_root_address, test_helpers::transaction_test_helpers};
use movement_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
};
use once_cell::sync::Lazy;
use rayon::vec;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing;
use url::Url;
use std::process::Command;
use std::{env, fs};

static SUZUKA_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	let config = dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap();
	config
});

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

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let rest_client = Client::new(NODE_URL.clone());
	let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone());
	let coin_client = CoinClient::new(&rest_client);
	let dead_address = AccountAddress::from_str(
		"000000000000000000000000000000000000000000000000000000000000dead")?;
	let relayer_address = AccountAddress::from_str(
		"0x000000000000000000000000000000000000000000000000000000000a550c18")?;
	let chain_id = rest_client
		.get_index()
		.await
		.context("failed to get chain ID")?
		.inner()
		.chain_id;

	// Create core resources account
	// let mut core_resources_account = LocalAccount::from_private_key(
	// 	SUZUKA_CONFIG
	// 		.execution_config
	// 		.maptos_config
	// 		.chain
	// 		.maptos_private_key
	// 		.to_encoded_string()?
	// 		.as_str(),
	// 	0,
	// )?;
	let mut core_resources_account: LocalAccount = LocalAccount::new(
        aptos_test_root_address(),
        AccountKey::from_private_key(aptos_vm_genesis::GENESIS_KEYPAIR.0.clone()),
        0,
    );
	
	println!("Core Resources Account address: {}", core_resources_account.address());

	tracing::info!("Created core resources account");
	tracing::debug!("core_resources_account address: {}", core_resources_account.address());

	// core_resources_account is already funded with u64 max value
	// Create dead account
	let create_dead_transaction = transaction_test_helpers::get_test_signed_transaction_with_chain_id(
        core_resources_account.address(),
        core_resources_account.sequence_number(),
        &aptos_vm_genesis::GENESIS_KEYPAIR.0,
        aptos_vm_genesis::GENESIS_KEYPAIR.1.clone(),
        Some(TransactionPayload::EntryFunction(EntryFunction::new(
			ModuleId::new(
				AccountAddress::from_hex_literal("0x1")?,
				Identifier::new("aptos_account")?,
			),
			Identifier::new("create_account")?,
			vec![],
			vec![bcs::to_bytes(&dead_address)?],
		))),
		SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
		100,
		None,
		ChainId::new(chain_id),
    );
    // let ret = vm_validator.validate_transaction(transaction).unwrap();

	// let create_dead_transaction =
	// 	core_resources_account.sign_with_transaction_builder(TransactionBuilder::new(
	// 		TransactionPayload::EntryFunction(EntryFunction::new(
	// 			ModuleId::new(
	// 				AccountAddress::from_hex_literal("0x1")?,
	// 				Identifier::new("aptos_account")?,
	// 			),
	// 			Identifier::new("create_account")?,
	// 			vec![],
	// 			vec![bcs::to_bytes(&dead_address)?],
	// 		)),
	// 		SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
	// 		ChainId::new(chain_id),
	// 	).sender(relayer_address).sequence_number(core_resources_account.sequence_number()));

	rest_client
		.submit_and_wait(&create_dead_transaction)
		.await
		.context("Failed to create dead account")?;

	coin_client
		.transfer(
			&mut core_resources_account,
			AccountAddress::from_str(
				"000000000000000000000000000000000000000000000000000000000000dead",
			)
			.unwrap(),
			1,
			None,
		)
		.await
		.context("Failed to transfer coins to dead account")?;

	// Retrieve and log balances
	let dead_balance = coin_client
		.get_account_balance(&dead_address)
		.await
		.context("Failed to retrieve dead account balance")?;
	let core_balance = coin_client
		.get_account_balance(&core_resources_account.address())
		.await
		.context("Failed to retrieve core resources account balance")?;

	// assert_eq!(core_resorces_balance, 999_999_999_999_999, "Core resources account balance is not what is expected");
	// assert_eq!(dead_balance, 1, "Dead account balance is not what is expected");

	tracing::info!(
		"Core account balance: {}, Dead account balance: {}",
		core_balance,
		dead_balance
	);

	let compile_status = Command::new("movement")
		.args([
			"move",
			"compile",
			"--package-dir",
			"protocol-units/bridge/move-modules",
		])
		.status()
		.expect("Failed to execute `movement compile` command");


	let code = fs::read("protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/main.mv")?;
	let args = vec![TransactionArgument::Address(dead_address), TransactionArgument::U64(1)];
	let script_payload = TransactionPayload::Script(Script::new(code, vec![], args));

	rest_client.submit_and_wait(&core_resources_account.sign_with_transaction_builder(
		TransactionBuilder::new(
			script_payload,
			SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
			ChainId::new(chain_id),
		).sender(relayer_address).sequence_number(core_resources_account.sequence_number())
	)).await.context("Failed to execute burn dead balance script transaction")?;

	// transfer to relayer address the desired amount
	// let desired_amount = 1_000_000;
	// coin_client
	// 	.transfer(
	// 		&mut core_resources_account,
	// 		AccountAddress::from_str(
	// 			"000000000000000000000000000000000000000000000000000000000a550c18",
	// 		)
	// 		.unwrap(),
	// 		desired_amount,
	// 		None,
	// 	)
	// 	.await
	// 	.context("Failed to transfer coins to relayer account")?;

	// reset core_balance
	// core_balance = coin_client
	// 	.get_account_balance(&core_resources_account.address())
	// 	.await
	// 	.context("Failed to retrieve core resources account balance")?;

	// // Burn coins from the core resource account
	// let code = fs::read("protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/main.mv")?;
	// let args = vec![TransactionArgument::Address(core_resources_account.address()), TransactionArgument::U64(core_balance)];
	// let script_payload = TransactionPayload::Script(Script::new(code, vec![], args));
	
	// rest_client.submit_and_wait(&core_resources_account.sign_with_transaction_builder(
	// 	TransactionBuilder::new(
	// 		script_payload,
	// 		SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
	// 		ChainId::new(chain_id),
	// 	).sequence_number(core_resources_account.sequence_number())
	// )).await.context("Failed to execute burn dead balance script transaction")?;

	tracing::info!("Burn transactions successfully executed.");


	// Transfer L1 move desired amount to L1 bridge address
	

	
	// Check if Relayer address balance on L2 equals to L1 bridge address
	

	Ok(())
}
