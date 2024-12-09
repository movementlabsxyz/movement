use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::HarnessMvtClient;
use bridge_integration_tests::TestHarness;
use bridge_service::chains::movement::event_monitoring::MovementMonitoring;
use bridge_service::{
	chains::{ethereum::types::EthAddress, movement::utils::MovementAddress},
	types::{Amount, BridgeAddress},
};
use bridge_util::BridgeClientContract;
use bridge_util::BridgeContractEvent;
use bridge_util::BridgeRelayerContract;
use futures::StreamExt;
use tokio::{self};

#[tokio::test]
async fn test_movement_client_initiate_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut mvt_client_harness, config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let (_mvt_health_tx, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let mut mvt_monitoring =
		MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();
	let recipient_address = HarnessEthClient::get_recipient_address(&config).to_vec();

	let test_result = async {
		mvt_client_harness
			.fund_signer_and_check_balance_framework(100_000_000_000)
			.await?;
		{
			tracing::info!("Before initiate_bridge_transfer");
			let res = BridgeClientContract::initiate_bridge_transfer(
				&mut mvt_client_harness.movement_client,
				BridgeAddress(recipient_address.clone()),
				Amount(100_000_000_000),
			)
			.await?;

			tracing::info!("Initiate result: {:?}", res);
		}

		let bridge_fee = mvt_client_harness.get_bridge_fee().await?;

		// Wait for the Movement Initiated event
		tracing::info!("Wait for Movement-side Initiated event.");
		let mut received_event = None;

		// Use a loop to wait for the Initiated event
		loop {
			let event =
				tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next())
					.await
					.expect("Wait for initiated event timeout.");

			if let Some(Ok(BridgeContractEvent::Initiated(detail))) = event {
				tracing::info!("Initiated details: {:?}", detail);

				received_event = Some((
					detail.bridge_transfer_id,
					detail.initiator,
					detail.recipient,
					detail.amount,
					detail.nonce,
				));
				break;
			}
		}

		let (bridge_transfer_id, initiator, recipient, amount, _nonce) =
			received_event.expect("No initiated event received");

		tracing::info!("Received bridge_transfer_id: {:?}", bridge_transfer_id);

		assert_eq!(initiator.0 .0, mvt_client_harness.signer_address());
		assert_eq!(recipient, BridgeAddress(recipient_address));
		assert_eq!(amount, Amount(100_000_000_000 - bridge_fee));

		Ok(())
	}
	.await;

	test_result
}

#[tokio::test]
async fn test_movement_client_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut mvt_client_harness, config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let (_mvt_health_tx, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let mut mvt_monitoring =
		MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

	// Set initiator as bytes
	let initiator = EthAddress(HarnessEthClient::get_initiator_address(&config));

	// Set recipient address
	let recipient = HarnessMvtClient::gen_aptos_account().address();

	// Set amount to 1
	let amount = Amount(100_000_000_000);

	// Random nonce
	let incoming_nonce = TestHarness::create_nonce();

        tracing::info!("Initiator: {:?}", initiator.clone().0);
        tracing::info!("Recipient: {:?}", recipient);
        tracing::info!("Amount: {:?}", amount);
        tracing::info!("Incoming nonce: {:?}", incoming_nonce);

	let bridge_transfer_id = HarnessMvtClient::calculate_bridge_transfer_id(
		initiator.clone().0,
		recipient,
		amount,
		incoming_nonce,
	);

	let coin_client = CoinClient::new(&mvt_client_harness.rest_client);
	let movement_client_signer = mvt_client_harness.movement_client.signer();

	// Fund accounts
	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
		faucet_client.fund(recipient, 100_000_000).await?;
	}

	// Assert the balance is sufficient
	let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
	assert!(
		balance >= 100_000_000,
		"Expected Movement Client to have at least 100_000_000, but found {}",
		balance
	);

	// Call the complete_bridge_transfer function
	BridgeRelayerContract::complete_bridge_transfer(
		&mut mvt_client_harness.movement_client,
		bridge_transfer_id,
		BridgeAddress(initiator.clone().to_vec()),
		BridgeAddress(MovementAddress(recipient)),
		amount,
		incoming_nonce,
	)
	.await
	.expect("Failed to complete bridge transfer");

	// Wait for the Movement-side Completed event
	tracing::info!("Wait for Movement-side Completed event.");
	loop {
		let event = tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next())
			.await
			.expect("Wait for completed event timeout.");
		if let Some(Ok(BridgeContractEvent::Completed(detail))) = event {
			tracing::info!("Completed details: {:?}", detail);
			assert_eq!(
				detail.bridge_transfer_id, bridge_transfer_id,
				"Bad transfer id in completed event"
			);
			assert_eq!(detail.nonce, incoming_nonce, "Bad nonce in completed event");
			assert_eq!(detail.amount, amount, "Bad amount in completed event");
			assert_eq!(
				detail.initiator,
				BridgeAddress(initiator.to_vec()),
				"Bad initiator address in completed event"
			);
			assert_eq!(
				detail.recipient.0 .0, recipient,
				"Bad recipient address in completed event"
			);
			break;
		}
	}

	Ok(())
}
