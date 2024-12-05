use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::HarnessMvtClient;
use bridge_integration_tests::TestHarness;
use bridge_service::chains::ethereum::event_monitoring::EthMonitoring;
use bridge_service::chains::ethereum::types::EthAddress;
use bridge_service::chains::movement::utils::MovementAddress;
use bridge_service::types::Amount;
use bridge_util::chains::bridge_contracts::BridgeRelayerContract;
use bridge_util::types::BridgeAddress;
use bridge_util::BridgeContractEvent;
use futures::StreamExt;
use tokio::{self};

#[tokio::test]
async fn test_eth_client_initiate_bridge_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");
	let (_eth_health_tx, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();

	let recipient = HarnessMvtClient::gen_aptos_account();
	let res = eth_client_harness
		.initiate_eth_bridge_transfer(
			&config,
			HarnessEthClient::get_initiator_private_key(&config),
			MovementAddress(recipient.address()),
			Amount(1),
		)
		.await;
	assert!(res.is_ok(), "initiate_bridge_transfer failed because: {res:?}");

	// Wait for the Eth-side Initiated event
	tracing::info!("Wait for Eth-side Initiated event.");
	loop {
		let event = tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next())
			.await
			.expect("Wait for completed event timeout.");
		if let Some(Ok(BridgeContractEvent::Initiated(detail))) = event {
			tracing::info!("Initiated details: {:?}", detail);
			let recipient_address =
				BridgeAddress(Into::<Vec<u8>>::into(MovementAddress(recipient.address())));
			assert_eq!(recipient_address, detail.recipient, "Bad recipient in Initiated event");
			break;
		}
	}
}

#[tokio::test]
async fn test_eth_client_complete_bridge_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();

	let (mut eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");
	let (_eth_health_tx, eth_health_rx) = tokio::sync::mpsc::channel(10);
	let mut eth_monitoring = EthMonitoring::build(&config.eth, eth_health_rx).await.unwrap();

	let initiator_address = HarnessMvtClient::gen_aptos_account().address();
	let recipeint_address =
		EthAddress(HarnessEthClient::get_recipient_private_key(&config).address());

	let nonce = TestHarness::create_nonce();
	let amount = Amount(2);

	let transfer_id = HarnessEthClient::calculate_bridge_transfer_id(
		initiator_address,
		*recipeint_address,
		amount,
		nonce,
	);

	tracing::info!("Transfer ID Eth side: {:?}", transfer_id);

	let res = eth_client_harness
		.eth_client
		.complete_bridge_transfer(
			transfer_id,
			BridgeAddress(initiator_address.into()),
			BridgeAddress(recipeint_address),
			amount,
			nonce,
		)
		.await;

	assert!(res.is_ok(), "complete_bridge_transfer failed: {:?}", res.unwrap_err());

	// Wait for the Eth-side Completed event
	tracing::info!("Wait for Eth-side Completed event.");
	loop {
		let event = tokio::time::timeout(std::time::Duration::from_secs(30), eth_monitoring.next())
			.await
			.expect("Wait for completed event timeout.");
		if let Some(Ok(BridgeContractEvent::Completed(detail))) = event {
			tracing::info!("Completed details: {:?}", detail);
			assert_eq!(
				transfer_id, detail.bridge_transfer_id,
				"Bad transfer id in completed event"
			);
			assert_eq!(nonce, detail.nonce, "Bad nonce in completed event");
			break;
		}
	}
}
