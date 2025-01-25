#![allow(unused_imports)]
use anyhow::{Chain, Context};
use movement_client::crypto::ValidCryptoMaterialStringExt;

use aptos_sdk::types::{
	account_address::AccountAddress,
	chain_id::ChainId,
	transaction::{EntryFunction, Script, TransactionArgument},
	AccountKey, LocalAccount,
};
use aptos_sdk::{
	crypto::ed25519::ed25519_keys::Ed25519PublicKey,
	crypto::multi_ed25519::MultiEd25519PublicKey,
	move_types::{
		identifier::Identifier,
		language_storage::{ModuleId, StructTag, TypeTag},
	},
	rest_client::Account,
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
use aptos_types::{
	account_config::aptos_test_root_address, test_helpers::transaction_test_helpers,
	transaction::TransactionPayload,
};
use movement_client::{
	coin_client::CoinClient,
	rest_client::{Client, FaucetClient},
};
use once_cell::sync::Lazy;
use rayon::vec;
use std::process::Command;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};
use tracing;
use url::Url;

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
		"000000000000000000000000000000000000000000000000000000000000dead",
	)?;
	let associate_address = AccountAddress::from_str(
		"0x000000000000000000000000000000000000000000000000000000000a550c18",
	)?;

	let chain_id = rest_client
		.get_index()
		.await
		.context("failed to get chain ID")?
		.inner()
		.chain_id;

	let mut core_resources_account = LocalAccount::from_private_key(
		SUZUKA_CONFIG
			.execution_config
			.maptos_config
			.chain
			.maptos_private_key
			.to_encoded_string()?
			.as_str(),
		0,
	)?;
	println!(
		"resource account keypairs: {:?}, {:?}",
		core_resources_account.private_key(),
		core_resources_account.public_key()
	);
	println!("Core Resources Account address: {}", core_resources_account.address());

	println!("Created core resources account");

	// core_resources_account is already funded with u64 max value
	// Create dead account
	let create_dead_transaction =
		transaction_test_helpers::get_test_signed_transaction_with_chain_id(
			associate_address,
			core_resources_account.sequence_number(),
			&core_resources_account.private_key(),
			core_resources_account.public_key().clone(),
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

	rest_client
		.submit_and_wait(&create_dead_transaction)
		.await
		.context("Failed to create dead account")?;

	core_resources_account.increment_sequence_number();

	faucet_client.fund(core_resources_account.address(), 1_000_000).await?;
	core_resources_account.increment_sequence_number();
	faucet_client.fund(dead_address, 1).await?;
	core_resources_account.increment_sequence_number();

	// Retrieve and log balances
	let dead_balance = coin_client
		.get_account_balance(&dead_address)
		.await
		.context("Failed to retrieve dead account balance")?;
	let core_balance = coin_client
		.get_account_balance(&core_resources_account.address())
		.await
		.context("Failed to retrieve core resources account balance")?;

	println!("Core account balance: {}, Dead account balance: {}", core_balance, dead_balance);

	Command::new("movement")
		.args(["move", "compile", "--package-dir", "protocol-units/bridge/move-modules"])
		.status()
		.expect("Failed to execute `movement compile` command");

	let enable_bridge_code = fs::read("protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/enable_bridge_feature.mv")?;
	let enable_bridge_script_payload =
		TransactionPayload::Script(Script::new(enable_bridge_code, vec![], vec![]));

	let enable_bridge_script_transaction =
		transaction_test_helpers::get_test_signed_transaction_with_chain_id(
			associate_address,
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

	let store_mint_burn_caps_code = fs::read("protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/store_mint_burn_caps.mv")?;
	let store_mint_burn_caps_script_payload =
		TransactionPayload::Script(Script::new(store_mint_burn_caps_code, vec![], vec![]));

	let store_mint_burn_caps_script_transaction =
		transaction_test_helpers::get_test_signed_transaction_with_chain_id(
			associate_address,
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

	println!("Bridge feature enabled and mint burn caps stored");

	let burn_dead_code = fs::read(
		"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/burn_from.mv",
	)?;
	let burn_dead_args =
		vec![TransactionArgument::Address(dead_address), TransactionArgument::U64(1)];
	let burn_dead_script_payload =
		TransactionPayload::Script(Script::new(burn_dead_code, vec![], burn_dead_args));
	let burn_dead_script_transaction =
		transaction_test_helpers::get_test_signed_transaction_with_chain_id(
			associate_address,
			core_resources_account.sequence_number(),
			&core_resources_account.private_key(),
			core_resources_account.public_key().clone(),
			Some(burn_dead_script_payload),
			SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
			100,
			None,
			ChainId::new(chain_id),
		);

	rest_client
		.submit_and_wait(&burn_dead_script_transaction)
		.await
		.context("Failed to execute burn dead balance script transaction")?;

	core_resources_account.increment_sequence_number();

	let desired_core_balance = 1;

	let amount_to_burn = core_balance - desired_core_balance;

	let burn_core_code = fs::read(
		"protocol-units/bridge/move-modules/build/bridge-modules/bytecode_scripts/burn_from.mv",
	)?;
	let burn_core_args =
		vec![TransactionArgument::Address(core_resources_account.address()), TransactionArgument::U64(amount_to_burn)];
	let burn_core_script_payload =
		TransactionPayload::Script(Script::new(burn_core_code, vec![], burn_core_args));
	let burn_core_script_transaction =
		transaction_test_helpers::get_test_signed_transaction_with_chain_id(
			associate_address,
			core_resources_account.sequence_number(),
			&core_resources_account.private_key(),
			core_resources_account.public_key().clone(),
			Some(burn_core_script_payload),
			SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 60,
			100,
			None,
			ChainId::new(chain_id),
		);

	rest_client
		.submit_and_wait(&burn_core_script_transaction)
		.await
		.context("Failed to execute burn dead balance script transaction")?;

	core_resources_account.increment_sequence_number();

	println!("Script burn transactions successfully executed.");

	assert!(
		coin_client
			.get_account_balance(&core_resources_account.address())
			.await
			.context("Failed to retrieve core resources account new balance")?
			== desired_core_balance
	);
	// Transfer L1 move desired amount to L1 bridge address
	// not needed to cover as its a simple bridge, what matters is transferring the correct amount without initiating a bridge attempt
	// Manual check of if Relayer address balance on L2 equals to L1 bridge address

	Ok(())
}
