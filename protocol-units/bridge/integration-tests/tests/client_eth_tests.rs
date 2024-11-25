use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::HarnessMvtClient;
use bridge_integration_tests::TestHarness;
use bridge_service::chains::ethereum::event_monitoring::EthMonitoring;
use bridge_service::chains::ethereum::types::EthAddress;
use bridge_service::types::Amount;
use bridge_service::types::Nonce;
use bridge_util::chains::bridge_contracts::BridgeRelayerContract;
use bridge_util::types::BridgeAddress;
use bridge_util::BridgeContractEvent;
use bridge_util::BridgeTransferId;
use futures::StreamExt;
use tokio::{self};

#[tokio::test]
async fn test_eth_client_initiate_bridge_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	let recipient = HarnessMvtClient::gen_aptos_account();
	let res = eth_client_harness
		.initiate_eth_bridge_transfer(
			&config,
			HarnessEthClient::get_initiator_private_key(&config),
			bridge_service::chains::movement::utils::MovementAddress(recipient.address()),
			Amount(1),
		)
		.await;
	assert!(res.is_ok(), "initiate_bridge_transfer failed because: {res:?}");
}

#[tokio::test]
async fn test_eth_client_complete_bridge_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();

	let (mut eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");
	let (_eth_health_tx, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();

	let transfer_id = BridgeTransferId::gen_unique_hash(&mut rand::rngs::OsRng);
	let initiator_address =
		EthAddress(HarnessEthClient::get_initiator_private_key(&config).address());
	let recipeint_address =
		EthAddress(HarnessEthClient::get_recipient_private_key(&config).address());
	let nonce = Nonce(1);

	let res = eth_client_harness
		.eth_client
		.complete_bridge_transfer(
			transfer_id,
			BridgeAddress(initiator_address.into()),
			BridgeAddress(recipeint_address),
			Amount(1),
			nonce,
		)
		.await;

	assert!(res.is_ok(), "initiate_bridge_transfer failed: {:?}", res.unwrap_err());

	// Wait for the tx to be executed
	tracing::info!("Wait for the ETH Initiated event.");

	// Use timeout to wait for the next event
	let event_option =
		tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next())
			.await
			.expect("Timeout while waiting for the ETH Locked event");

	// Check if we received an event (Option) and handle the Result inside it
	match event_option {
		Some(Ok(BridgeContractEvent::Completed(detail))) => detail.bridge_transfer_id,
		Some(Err(e)) => panic!("Error in bridge contract event: {:?}", e),
		None => panic!("No event received"),
		_ => panic!("Not a an Initiated event: {:?}", event_option),
	};
}
