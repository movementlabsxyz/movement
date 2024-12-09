use alloy_primitives::Address;
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
	//tracing_subscriber::fmt().with_env_filter(EnvFilter::new("info")).init();

	let (eth_client_harness, mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;

	tracing::info!("Init initiator and counterparty test accounts.");
	tracing::info!("Use client signer for Mvt and index 2 of config.eth.eth_well_known_account_private_keys array for Eth");

	// Init mvt addresses
	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000_000).await?;
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
			let addr_vec: Vec<u8> = EthAddress(HarnessEthClient::get_initiator_address(&config)).into();
			assert_eq!(detail.initiator.0, addr_vec);
			assert_eq!(detail.recipient, BridgeAddress(recipient));
			assert_eq!(detail.amount, amount);
			assert_eq!(detail.nonce, nonce);

			break;
		}
	}

	Ok(())
}

#[tokio::test]
async fn test_bridge_transfer_movement_eth_happy_path() -> Result<(), anyhow::Error> {
	// tracing_subscriber::fmt()
	// 	.with_env_filter(
	// 		EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
	// 	)
	// 	.init();

	let (mut eth_client_harness, mut mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;
	// must include name of sender channel to avoid it being dropped
	let (_mvt_health_tx, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let mut mvt_monitoring =
		MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

	// Init mvt addresses
	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();
	let initiator_privkey = mvt_client_harness.fund_account().await;
	let initiator_address = MovementAddress(initiator_privkey.address());
	tracing::info!("Initiator address: {:?}", initiator_address);
	let recipient_address = HarnessEthClient::get_recipient_address(&config).to_vec();
	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000_000_000).await?;
		faucet_client.fund(initiator_privkey.address(), 100_000_000_000_000).await?;
	}
	let bridge_fee = mvt_client_harness.get_bridge_fee().await?;

	tracing::info!("Before initiate_bridge_transfer");
	let res = BridgeClientContract::initiate_bridge_transfer(
		&mut mvt_client_harness.movement_client,
		BridgeAddress(recipient_address.clone()),
		Amount(100_000_000_000),
	)
	.await?;

	tracing::info!("Initiate result: {:?}", res);

	// Wait for the Movement-side Initiated event
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
	let (_eth_health_tx, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();
	loop {
		let event =
			tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next()).await?;
		if let Some(Ok(BridgeContractEvent::Completed(detail))) = event {
			assert_eq!(detail.bridge_transfer_id, bridge_transfer_id);
			// assert_eq!(detail.initiator.0, initiator_address.0.to_vec());
			assert_eq!(detail.recipient.0.0.to_vec(), recipient_address);
			assert_eq!(detail.amount, Amount(100_000_000_000 - bridge_fee));
			assert_eq!(detail.nonce, nonce);
			break;
		}
	}

	Ok(())
}