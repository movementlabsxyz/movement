use aptos_sdk::rest_client::Resource;
use aptos_sdk::types::transaction::TransactionArgument;
use aptos_sdk::{
	rest_client::Client,
	transaction_builder::TransactionFactory,
	types::{account_address::AccountAddress, transaction::TransactionPayload},
};
use aptos_types::{chain_id::ChainId, transaction::Script};
use movement_client::types::{account_config::aptos_test_root_address, LocalAccount};
use once_cell::sync::Lazy;
use std::str::FromStr;
use url::Url;

static MOVEMENT_CONFIG: Lazy<movement_config::Config> = Lazy::new(|| {
	let dot_movement = dot_movement::DotMovement::try_from_env().unwrap();
	dot_movement.try_get_config_from_json::<movement_config::Config>().unwrap()
});

static NODE_URL: Lazy<Url> = Lazy::new(|| {
	let addr = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_hostname
		.clone();
	let port = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.client
		.maptos_rest_connection_port
		.clone();
	Url::from_str(&format!("http://{}:{}", addr, port)).unwrap()
});

const GAS_UNIT_LIMIT: u64 = 100_000;

const CHANGE_EPOCH_MV: &[u8] = include_bytes!("../move/build/change_epoch.mv");

pub async fn set_epoch_duration(epoch_duration: u64) -> Result<(), anyhow::Error> {
	println!("MVT_NODE_REST_URL: {:?}", std::env::var("MVT_NODE_REST_URL"));
	let node_url = std::env::var("MVT_NODE_REST_URL")
		.as_ref()
		.map(|url| Url::from_str(url))
		.unwrap_or(Ok(NODE_URL.clone()))?;
	println!("node_url:{node_url}");
	let rest_client = Client::new(node_url);

	// Core resources (aptos_test_root) address
	let gov_root_address = aptos_test_root_address();
	tracing::info!("aptos_test_root_address() (constant): {}", gov_root_address);
	// Load *core_resources* private key (from your config/genesis)
	let raw_private_key = MOVEMENT_CONFIG
		.execution_config
		.maptos_config
		.chain
		.maptos_private_key_signer_identifier
		.try_raw_private_key()?;
	let gov_priv =
		movement_client::crypto::ed25519::Ed25519PrivateKey::try_from(raw_private_key.as_slice())?;

	// Build signer by *forcing* core_resources address + current on-chain seq
	let gov_root_account = {
		let onchain = rest_client.get_account(gov_root_address).await?.into_inner();
		LocalAccount::new(gov_root_address, gov_priv.clone(), onchain.sequence_number)
	};
	tracing::info!("Signer (gov_root_account) address: {}", gov_root_account.address());

	let ledger_info = rest_client.get_ledger_information().await?.into_inner();
	let factory = TransactionFactory::new(ChainId::new(ledger_info.chain_id))
		.with_gas_unit_price(100)
		.with_max_gas_amount(GAS_UNIT_LIMIT);

	let payload = TransactionPayload::Script(Script::new(
		CHANGE_EPOCH_MV.to_vec(),
		vec![],                                         // no type args
		vec![TransactionArgument::U64(epoch_duration)], // New epoch duration.
	));

	// Sign & submit
	let signed_txn = gov_root_account.sign_with_transaction_builder(factory.payload(payload));

	let tx_hash = rest_client.submit(&signed_txn).await?.into_inner();
	let res = rest_client.wait_for_transaction(&tx_hash).await?.into_inner();

	// Verify the epoch has been changed
	let block_res: Resource = rest_client
		.get_account_resource(AccountAddress::from_hex_literal("0x1")?, "0x1::block::BlockResource")
		.await?
		.into_inner()
		.unwrap();

	let interval_str = block_res.data["epoch_interval"]
		.as_str()
		.ok_or_else(|| anyhow::anyhow!("epoch_interval missing or not a string"))?;

	let onchain_duration: u64 = interval_str.parse()?;

	assert!(
		onchain_duration == epoch_duration,
		"Epoch duration not updated, epoch after update is:{onchain_duration}"
	);

	tracing::info!(
		"✅ Executed change epoch script, new epoch duration:{onchain_duration}, with Tx hash: {}",
		res.transaction_info().unwrap().hash
	);
	Ok(())
}
