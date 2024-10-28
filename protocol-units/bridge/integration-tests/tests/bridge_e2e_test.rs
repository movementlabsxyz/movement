use alloy::primitives::keccak256;
use alloy::primitives::{FixedBytes, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use anyhow::Result;
use aptos_types::account_address::AccountAddress;
use bridge_config::Config;
use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::TestHarness;
use bridge_service::chains::bridge_contracts::BridgeContractError;
use bridge_service::chains::bridge_contracts::BridgeContractEvent;
use bridge_service::chains::ethereum::event_monitoring::EthMonitoring;
use bridge_service::chains::ethereum::types::AtomicBridgeInitiator;
use bridge_service::chains::ethereum::utils::send_transaction;
use bridge_service::chains::ethereum::utils::send_transaction_rules;
use bridge_service::chains::{
	ethereum::types::EthAddress,
	movement::{
		client::MovementClient, event_monitoring::MovementMonitoring, utils::MovementAddress,
	},
};
use bridge_service::types::Amount;
use bridge_service::types::BridgeAddress;
use bridge_service::types::HashLock;
use bridge_service::types::HashLockPreImage;
use futures::StreamExt;
use tracing_subscriber::EnvFilter;

async fn initiate_eth_bridge_transfer(
	config: &Config,
	initiator_privatekey: PrivateKeySigner,
	recipient: MovementAddress,
	hash_lock: HashLock,
	amount: Amount,
) -> Result<(), anyhow::Error> {
	let initiator_address = initiator_privatekey.address();
	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(initiator_privatekey))
		.on_builtin(&config.eth.eth_rpc_connection_url())
		.await?;

	let contract =
		AtomicBridgeInitiator::new(config.eth.eth_initiator_contract.parse()?, &rpc_provider);

	let initiator_address = BridgeAddress(EthAddress(initiator_address));

	let recipient_address = BridgeAddress(Into::<Vec<u8>>::into(recipient));

	let recipient_bytes: [u8; 32] =
		recipient_address.0.try_into().expect("Recipient address must be 32 bytes");

	let call = contract
		.initiateBridgeTransfer(
			U256::from(amount.0),
			FixedBytes(recipient_bytes),
			FixedBytes(hash_lock.0),
		)
		.value(U256::from(amount.0))
		.from(*initiator_address.0);

	let _ = send_transaction(
		call,
		&send_transaction_rules(),
		config.eth.transaction_send_retries,
		config.eth.gas_limit as u128,
	)
	.await
	.map_err(|e| BridgeContractError::GenericError(format!("Failed to send transaction: {}", e)))?;
	Ok(())
}

#[tokio::test]
async fn test_bridge_transfer_eth_movement_happy_path() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let (_eth_client_harness, mut mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;

	tracing::info!("Init initiator and counter part test account.");
	tracing::info!("Use client signer for Mvt and index 2 of config.eth.eth_well_known_account_private_keys array for Eth");

	// Init mvt addresses
	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000).await?;
	}

	let recipient_privkey = mvt_client_harness.fund_account().await;
	let recipient_address = MovementAddress(recipient_privkey.address());

	// 1) initialize Eth transfer
	tracing::info!("Call initiate_transfer on Eth");
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let amount = Amount(1000);
	initiate_eth_bridge_transfer(
		&config,
		HarnessEthClient::get_initiator_private_key(&config),
		recipient_address,
		hash_lock,
		amount,
	)
	.await
	.expect("Failed to initiate bridge transfer");

	//Wait for the tx to be executed
	tracing::info!("Wait for the MVT Locked event.");
	let mut mvt_monitoring = MovementMonitoring::build(&config.movement).await.unwrap();
	let event =
		tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next()).await?;
	let bridge_tranfer_id = if let Some(Ok(BridgeContractEvent::Locked(detail))) = event {
		detail.bridge_transfer_id
	} else {
		panic!("Not a Locked event: {event:?}");
	};

	println!("bridge_tranfer_id : {:?}", bridge_tranfer_id);
	println!("hash_lock_pre_image : {:?}", hash_lock_pre_image);

	//send counter complete event.
	tracing::info!("Call counterparty_complete_bridge_transfer on MVT.");
	mvt_client_harness
		.counterparty_complete_bridge_transfer(
			recipient_privkey,
			bridge_tranfer_id,
			hash_lock_pre_image,
		)
		.await?;

	let mut eth_monitoring = EthMonitoring::build(&config.eth).await.unwrap();
	// Wait for InitialtorCompleted event
	tracing::info!("Wait for InitialtorCompleted event.");
	loop {
		let event =
			tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next()).await?;
		if let Some(Ok(BridgeContractEvent::InitialtorCompleted(_))) = event {
			break;
		}
	}

	Ok(())
}

#[tokio::test]
async fn test_movement_event() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	println!("Start test_movement_event",);

	let config = TestHarness::read_bridge_config().await?;
	println!("after test_movement_event",);

	use bridge_integration_tests::MovementToEthCallArgs;

	let mut movement_client = MovementClient::new(&config.movement).await.unwrap();

	let args = MovementToEthCallArgs::default();
	bridge_integration_tests::utils::initiate_bridge_transfer_helper(
		&mut movement_client,
		args.initiator.0,
		args.recipient.clone(),
		args.hash_lock.0,
		args.amount,
		true,
	)
	.await
	.expect("Failed to initiate bridge transfer");

	//Wait for the tx to be executed
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
	let event_type = format!(
		"{}::atomic_bridge_initiator::BridgeTransferStore",
		config.movement.movement_native_address
	);

	let res = test_get_events_by_account_event_handle(
		&config.movement.mvt_rpc_connection_url(),
		&config.movement.movement_native_address,
		&event_type,
	)
	.await;
	println!("res: {res:?}",);

	let res = fetch_account_events(
		&config.movement.mvt_rpc_connection_url(),
		&config.movement.movement_native_address,
		&event_type,
	)
	.await;
	println!("res: {res:?}",);

	Ok(())
}

async fn test_get_events_by_account_event_handle(
	rest_url: &str,
	account_address: &str,
	event_type: &str,
) {
	let url = format!(
		"{}/v1/accounts/{}/events/{}/bridge_transfer_initiated_events",
		rest_url, account_address, event_type
	);

	println!("url: {:?}", url);
	let client = reqwest::Client::new();

	// Send the GET request
	let response = client
		.get(&url)
		.query(&[("start", "0"), ("limit", "10")])
		.send()
		.await
		.unwrap()
		.text()
		.await;

	println!("Account direct response: {response:?}",);
}

use aptos_sdk::rest_client::Client;
use std::str::FromStr;

async fn fetch_account_events(rest_url: &str, account_address: &str, event_type: &str) {
	// Initialize the RestClient
	let node_connection_url = url::Url::from_str(rest_url).unwrap();
	let client = Client::new(node_connection_url); // Use the correct node URL
	let native_address = AccountAddress::from_hex_literal(account_address).unwrap();

	// Get the events for the specified account
	let response = client
		.get_account_events(
			native_address,
			event_type,
			"bridge_transfer_initiated_events",
			Some(1),
			None,
		)
		.await;

	println!("response{response:?}",);
}
