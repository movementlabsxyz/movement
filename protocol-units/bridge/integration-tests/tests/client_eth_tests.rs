use alloy::primitives::keccak256;
use anyhow::Result;
use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::HarnessMvtClient;
use bridge_integration_tests::TestHarness;
use bridge_service::chains::bridge_contracts::{BridgeContract, BridgeContractEvent};
use bridge_service::chains::ethereum::{
	event_monitoring::EthMonitoring,
	types::EthAddress
};
use bridge_service::types::{Amount, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage};
use futures::StreamExt;
use tokio::{self};

#[tokio::test]
async fn test_eth_client_counterparty_complete_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	// initialize Eth transfer
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let amount = Amount(1);

	let transfer_id = BridgeTransferId::gen_unique_hash(&mut rand::rngs::OsRng);
	let res = BridgeContract::lock_bridge_transfer(
		&mut eth_client_harness.eth_client,
		transfer_id,
		hash_lock,
		BridgeAddress(vec![3; 32]),
		BridgeAddress(EthAddress(HarnessEthClient::get_recipeint_address(&config))),
		amount,
	)
	.await;
	assert!(res.is_ok());

	BridgeContract::counterparty_complete_bridge_transfer(
		&mut eth_client_harness.eth_client,
		transfer_id,
		hash_lock_pre_image,
	)
	.await
	.expect("Failed to complete bridge transfer");
}

#[tokio::test]
async fn test_eth_client_lock_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	// Call lock transfer Eth
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let amount = Amount(1);
	let transfer_id = BridgeTransferId::gen_unique_hash(&mut rand::rngs::OsRng);

	let res = eth_client_harness
		.eth_client
		.lock_bridge_transfer(
			transfer_id,
			hash_lock,
			BridgeAddress(vec![3; 32]),
			BridgeAddress(EthAddress(HarnessEthClient::get_recipeint_address(&config))),
			amount,
		)
		.await;

	assert!(res.is_ok(), "lock_bridge_transfer failed because: {res:?}");

	Ok(())
}

#[tokio::test]
async fn test_eth_client_initiate_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	let recipient = HarnessMvtClient::gen_aptos_account();
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));

	let res = eth_client_harness
		.initiate_eth_bridge_transfer(
			&config,
			HarnessEthClient::get_initiator_private_key(&config),
			bridge_service::chains::movement::utils::MovementAddress(recipient.address()),
			hash_lock,
			Amount(1),
		)
		.await;
	assert!(res.is_ok(), "initiate_bridge_transfer failed because: {res:?}");
}

#[tokio::test]
async fn test_eth_client_initiator_complete_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();

	let (eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	let recipient = HarnessMvtClient::gen_aptos_account();
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));

	let res = eth_client_harness
		.initiate_eth_bridge_transfer(
		&config,
		HarnessEthClient::get_initiator_private_key(&config),
		bridge_service::chains::movement::utils::MovementAddress(recipient.address()),
		hash_lock,
		Amount(1),
		)
		.await;

	assert!(res.is_ok(), "initiate_bridge_transfer failed: {:?}", res.unwrap_err());

	// Wait for the tx to be executed
	tracing::info!("Wait for the ETH Initiated event.");
	let (_, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();

	// Use timeout to wait for the next event
	let event_option = tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next())
		.await
		.expect("Timeout while waiting for the ETH Locked event");

	// Check if we received an event (Option) and handle the Result inside it
	let bridge_transfer_id = match event_option {
		Some(Ok(BridgeContractEvent::Initiated(detail))) => {
		detail.bridge_transfer_id
		},
		Some(Err(e)) => panic!("Error in bridge contract event: {:?}", e),
		None => panic!("No event received"),
		_ => panic!("Not a an Initiated event: {:?}", event_option),
	};

	tracing::info!("Received bridge_transfer_id: {:?}", bridge_transfer_id);

	let res = eth_client_harness
		.initiator_complete_bridge_transfer(
		&config,
		HarnessEthClient::get_initiator_private_key(&config),
		bridge_transfer_id,
		hash_lock_pre_image,
		)
		.await;

	assert!(res.is_ok(), "initiator_complete_bridge_transfer failed: {:?}", res.unwrap_err());

}

#[tokio::test]
async fn test_eth_client_refund_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();

	let (mut eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	let recipient = HarnessMvtClient::gen_aptos_account();
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));

	let res = eth_client_harness
		.initiate_eth_bridge_transfer(
		&config,
		HarnessEthClient::get_initiator_private_key(&config),
		bridge_service::chains::movement::utils::MovementAddress(recipient.address()),
		hash_lock,
		Amount(1),
		)
		.await;

	assert!(res.is_ok(), "initiate_bridge_transfer failed: {:?}", res.unwrap_err());

	// Wait for the tx to be executed
	tracing::info!("Wait for the ETH Initiated event.");
	let (_, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();

	// Use timeout to wait for the next event
	let event_option = tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next())
		.await
		.expect("Timeout while waiting for the ETH Initiated event");

	// Check if we received an event (Option) and handle the Result inside it
	let bridge_transfer_id = match event_option {
		Some(Ok(BridgeContractEvent::Initiated(detail))) => {
		detail.bridge_transfer_id
		},
		Some(Err(e)) => panic!("Error in bridge contract event: {:?}", e),
		None => panic!("No event received"),
		_ => panic!("Not a an Initiated event: {:?}", event_option),
	};

	tracing::info!("Received bridge_transfer_id: {:?}", bridge_transfer_id);

	let res = eth_client_harness.eth_client
		.refund_bridge_transfer(
			bridge_transfer_id
		)
		.await;

	assert!(res.is_ok(), "initiator_complete_bridge_transfer failed: {:?}", res.unwrap_err());

}

