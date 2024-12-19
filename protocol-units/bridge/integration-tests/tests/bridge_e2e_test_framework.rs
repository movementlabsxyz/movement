use alloy::primitives::keccak256;
use anyhow::Result;
use aptos_types::account_address::AccountAddress;
use bridge_integration_tests::{HarnessEthClient, TestHarness};
use bridge_service::{
	chains::{
		bridge_contracts::{BridgeContract, BridgeContractEvent},
		ethereum::{event_monitoring::EthMonitoring, types::EthAddress},
		movement::{
			client_framework::MovementClientFramework, event_monitoring::MovementMonitoring,
			utils::MovementAddress,
		},
	},
	types::{Amount, BridgeAddress, HashLock, HashLockPreImage},
};
use futures::StreamExt;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_bridge_transfer_eth_movement_happy_path() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt().with_env_filter(EnvFilter::new("info")).init();

	let (eth_client_harness, mut mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;

	MovementClientFramework::bridge_setup_scripts().await?;

	tracing::info!("Init initiator and counterparty test accounts.");
	tracing::info!("Use client signer for Mvt and index 2 of config.eth.eth_well_known_account_private_keys array for Eth");

	// Init mvt addresses
	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000).await?;
	}

	let recipient_privkey = mvt_client_harness.fund_account().await;
	let recipient = MovementAddress(recipient_privkey.address());

	// initiate Eth transfer
	tracing::info!("Call initiate_transfer on Eth");
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let amount = Amount(1);
	eth_client_harness
		.initiate_eth_bridge_transfer(
			&config,
			HarnessEthClient::get_initiator_private_key(&config),
			recipient,
			hash_lock,
			amount,
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//Wait for the tx to be executed
	tracing::info!("Wait for the MVT Locked event.");
	let (_, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let mut mvt_monitoring =
		MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();
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

	let (_, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();

	// Wait for InitiatorCompleted event
	tracing::info!("Wait for InitiatorCompleted event.");
	loop {
		let event =
			tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next()).await?;
		if let Some(Ok(BridgeContractEvent::InitiatorCompleted(_))) = event {
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

	let mut movement_client = MovementClientFramework::new(&config.movement).await.unwrap();

	let args = MovementToEthCallArgs::default();

	{
		let res = BridgeContract::initiate_bridge_transfer(
			&mut movement_client,
			BridgeAddress(MovementAddress(args.initiator.0)),
			BridgeAddress(args.recipient.clone()),
			HashLock(args.hash_lock.0),
			Amount(args.amount),
		)
		.await?;

		tracing::info!("Initiate result: {:?}", res);
	}

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

#[tokio::test]
async fn test_bridge_transfer_movement_eth_happy_path() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	MovementClientFramework::bridge_setup_scripts().await?;

	let (mut eth_client_harness, mut mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;

	let (_, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();

	// Init mvt addresses
	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000).await?;
	}

	let recipient_privkey = mvt_client_harness.fund_account().await;
	let recipient_address = MovementAddress(recipient_privkey.address());

	// initiate Eth transfer
	tracing::info!("Call initiate_transfer on Eth and set up test");
	let hash_lock_pre_image = HashLockPreImage::random();
	//let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let amount = Amount(1);
	eth_client_harness
		.initiate_eth_bridge_transfer(
			&config,
			HarnessEthClient::get_initiator_private_key(&config),
			recipient_address,
			hash_lock,
			amount,
		)
		.await
		.expect("Failed to initiate bridge transfer");

	tracing::info!("Before init_set_timelock");

	//mvt_client_harness.init_set_timelock(60).await?; //Set to 1mn

	tracing::info!("Init initiator and counterparty test accounts.");

	// Init mvt addresses
	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000_000).await?;
	}

	let initiator_account = mvt_client_harness.fund_account().await;

	let counterpart_privekey = HarnessEthClient::get_initiator_private_key(&config);
	let counter_party_address = EthAddress(counterpart_privekey.address());

	// Initialize Movement transfer
	let hash_lock_movement_pre_image = HashLockPreImage::random();
	tracing::info!(
		"Hash lock pre-image for Movement initiate transfer: {:?}",
		hash_lock_movement_pre_image
	);
	let hash_lock_movement = HashLock(From::from(keccak256(hash_lock_movement_pre_image)));
	tracing::info!("Hash lock for Movement initiate transfer: {:?}", hash_lock_movement);
	let amount = 1;
	mvt_client_harness
		.initiate_bridge_transfer(
			&initiator_account,
			counter_party_address,
			hash_lock_movement,
			amount,
		)
		.await?;

	// Wait for the Eth-side lock event
	tracing::info!("Wait for Eth-side Lock event.");
	let bridge_tranfer_id;
	loop {
		let event =
			tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next()).await?;
		if let Some(Ok(BridgeContractEvent::Locked(detail))) = event {
			tracing::info!("Lock details: {:?}", detail);
			bridge_tranfer_id = detail.bridge_transfer_id;
			break;
		}
	}
	tracing::info!("Bridge transfer ID from Eth Lock event: {:?}", bridge_tranfer_id);

	// Complete transfer on Eth
	tracing::info!("Before Eth counterparty complete with preimage {:?}", hash_lock_pre_image);
	eth_client_harness
		.eth_client
		.counterparty_complete_bridge_transfer(bridge_tranfer_id, hash_lock_pre_image)
		.await?;

	let (_, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let mut mvt_monitoring =
		MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

	tracing::info!("Wait for InitiatorCompleted event.");
	loop {
		let event =
			tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next()).await?;
		if let Some(Ok(BridgeContractEvent::InitiatorCompleted(_))) = event {
			break;
		}
	}

	Ok(())
}
