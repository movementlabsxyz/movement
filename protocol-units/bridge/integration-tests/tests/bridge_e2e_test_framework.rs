use anyhow::Result;
use aptos_types::account_address::AccountAddress;
use bridge_integration_tests::{HarnessEthClient, TestHarness};
use bridge_service::chains::movement::client_framework::FRAMEWORK_ADDRESS;
use bridge_service::{
	chains::{
		ethereum::{event_monitoring::EthMonitoring, types::EthAddress},
		movement::{
			client_framework::MovementClientFramework, event_monitoring::MovementMonitoring,
			utils::MovementAddress,
		},
	},
	types::{Amount, BridgeAddress},
};
use bridge_util::BridgeClientContract;
use bridge_util::BridgeContractEvent;
use futures::StreamExt;
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn test_bridge_transfer_eth_movement_happy_path() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt().with_env_filter(EnvFilter::new("info")).init();

	let (eth_client_harness, mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;

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
	let amount = Amount(1);

	// initiate Eth transfer
	tracing::info!("Call initiate_transfer on Eth");

	let res = eth_client_harness
		.initiate_eth_bridge_transfer(
			&config,
			HarnessEthClient::get_initiator_private_key(&config),
			recipient.clone(),
			amount,
		)
		.await;

	assert!(res.is_ok(), "e2e test, Eth initiate transfer failed:{res:?}");

	//Wait for the tx to be executed
	tracing::info!("Wait for the Eth Initiated event.");
	let (_eth_healthtx, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();
	let event =
		tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next()).await?;
	let (bridge_transfer_id, nonce) =
		if let Some(Ok(BridgeContractEvent::Initiated(detail))) = event {
			(detail.bridge_transfer_id, detail.nonce)
		} else {
			panic!("Not an Initiated event: {event:?}");
		};

	let (_mvt_health_tx, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let mut mvt_monitoring =
		MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

	// Wait for InitiatorCompleted event
	tracing::info!("Wait for Completed event.");
	loop {
		let event =
			tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next()).await?;
		if let Some(Ok(BridgeContractEvent::Completed(detail))) = event {
			assert_eq!(detail.bridge_transfer_id, bridge_transfer_id);
			let addr_vec: Vec<u8> = EthAddress(HarnessEthClient::get_initiator(&config)).into();
			let addr_ascii_hex: Vec<u8> = hex::encode(addr_vec).into_bytes();
			assert_eq!(detail.initiator.0, addr_ascii_hex);
			assert_eq!(detail.recipient, BridgeAddress(recipient));
			assert_eq!(detail.amount, amount);
			assert_eq!(detail.nonce, nonce);

			break;
		}
	}

	Ok(())
}

#[tokio::test]
#[ignore]
async fn test_bridge_transfer_movement_eth_happy_path() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let (mut eth_client_harness, mut mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;
	let (_, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let mut mvt_monitoring =
		MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

	// Init mvt addresses
	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();
	let initiator_privkey = mvt_client_harness.fund_account().await;
	let initiator_address = MovementAddress(initiator_privkey.address());

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000).await?;
		faucet_client.fund(initiator_privkey.address(), 100_000_000).await?;
	}

	let recipient = HarnessEthClient::get_recipeint_address(&config);
	let amount = Amount(1);

	// initiate Eth transfer
	let mut initiator_mvt_client =
		MovementClientFramework::build_with_signer(initiator_privkey, &config.movement).await?;
	// Call using initiator private key.
	let recipient_vec: Vec<u8> = EthAddress(recipient).into();
	initiator_mvt_client
		.initiate_bridge_transfer(BridgeAddress(recipient_vec), amount)
		.await
		.expect("Failed to initiate bridge transfer");
	tracing::info!("Hash lock pre-image for Movement initiate transfer.");

	// Wait for the Eth-side lock event
	tracing::info!("Wait for Mvt-side Initiated event.");
	let bridge_transfer_id;
	let nonce;
	loop {
		let event =
			tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next()).await?;
		if let Some(Ok(BridgeContractEvent::Initiated(detail))) = event {
			tracing::info!("Initiated details: {:?}", detail);
			bridge_transfer_id = detail.bridge_transfer_id;
			nonce = detail.nonce;
			break;
		}
	}
	tracing::info!("Bridge transfer ID from Mvt Initiated event: {:?}", bridge_transfer_id);
	tracing::info!("Wait for Completed event.");
	let (_, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();
	loop {
		let event =
			tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next()).await?;
		if let Some(Ok(BridgeContractEvent::Completed(detail))) = event {
			assert_eq!(detail.bridge_transfer_id, bridge_transfer_id);
			let initiator_vec: Vec<u8> = initiator_address.into();
			assert_eq!(detail.initiator.0, initiator_vec);
			assert_eq!(detail.recipient, BridgeAddress(EthAddress(recipient)));
			assert_eq!(detail.amount, amount);
			assert_eq!(detail.nonce, nonce);
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

	let mut movement_client =
		MovementClientFramework::build_with_config(&config.movement).await.unwrap();

	let args = MovementToEthCallArgs::default();

	{
		let res = BridgeClientContract::initiate_bridge_transfer(
			&mut movement_client,
			BridgeAddress(args.recipient.clone()),
			Amount(args.amount),
		)
		.await?;

		tracing::info!("Initiate result: {:?}", res);
	}

	//Wait for the tx to be executed
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
	let event_type = format!("{}::native_bridge::BridgeEvents", FRAMEWORK_ADDRESS);

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
	let res = client
		.get(&url)
		.query(&[("start", "0"), ("limit", "10")])
		.send()
		.await
		.unwrap()
		.text()
		.await;
	println!("Account direct response: {res:?}",);
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
