use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::account_address::AccountAddress;
use bridge_integration_tests::utils as test_utils;
use bridge_integration_tests::{MovementToEthCallArgs, TestHarness};
use bridge_service::chains::movement::event_monitoring::MovementMonitoring;
use bridge_service::{
	chains::movement::utils::MovementAddress,
	types::{Amount, BridgeAddress, BridgeTransferId},
};
use bridge_util::types::Nonce;
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
	let args = MovementToEthCallArgs::default();

	let test_result = async {
		test_utils::fund_and_check_balance_framework(&mut mvt_client_harness, 100_000_000_000)
			.await?;

		{
			let res = BridgeClientContract::initiate_bridge_transfer(
				&mut mvt_client_harness.movement_client,
				BridgeAddress(args.recipient.clone()),
				Amount(args.amount),
			)
			.await?;

			tracing::info!("Initiate result: {:?}", res);
		}

		// Wait for the tx to be executed
		tracing::info!("Wait for the Movement Initiated event.");
		let (_, mvt_health_rx) = tokio::sync::mpsc::channel(10);
		let mut mvt_monitoring =
			MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

		// Use timeout to wait for the next event
		let event_option =
			tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next())
				.await
				.expect("Timeout while waiting for the Movement Initiated event");

		// Check if we received an event (Option) and handle the Result inside it
		let (bridge_transfer_id, nonce) = match event_option {
			Some(Ok(BridgeContractEvent::Initiated(detail))) => {
				(detail.bridge_transfer_id, detail.nonce)
			}
			Some(Err(e)) => panic!("Error in bridge contract event: {:?}", e),
			None => panic!("No event received"),
			_ => panic!("Not a an Initiated event: {:?}", event_option),
		};

		tracing::info!("Received bridge_transfer_id: {:?}", bridge_transfer_id);

		let details = BridgeClientContract::get_bridge_transfer_details(
			&mut mvt_client_harness.movement_client,
			bridge_transfer_id,
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.bridge_transfer_id, bridge_transfer_id);
		assert_eq!(details.initiator.0 .0, mvt_client_harness.signer_address());
		assert_eq!(details.recipient, BridgeAddress(args.recipient.clone()));
		assert_eq!(details.amount, Amount(args.amount));
		assert_eq!(details.nonce, nonce);

		Ok(())
	}
	.await;

	test_result
}

#[tokio::test]
async fn test_movement_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut mvt_client_harness, config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let (_mvt_health_tx, mvt_health_rx) = tokio::sync::mpsc::channel(10);
	let mut mvt_monitoring =
		MovementMonitoring::build(&config.movement, mvt_health_rx).await.unwrap();

	let transfer_id = BridgeTransferId::gen_unique_hash(&mut rand::rngs::OsRng);
	let initiator = b"32Be343B94f860124dC4fEe278FDCBD38C102D88".to_vec();
	let recipient = AccountAddress::new(*b"0x00000000000000000000000000face");
	let amount = Amount(1);
	let incoming_nonce = Nonce(5);

	let coin_client = CoinClient::new(&mvt_client_harness.rest_client);
	let movement_client_signer = mvt_client_harness.movement_client.signer();
	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
		faucet_client.fund(recipient, 100_000_000).await?;
	}
	let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
	assert!(
		balance >= 100_000_000,
		"Expected Movement Client to have at least 100_000_000, but found {}",
		balance
	);

	BridgeRelayerContract::complete_bridge_transfer(
		&mut mvt_client_harness.movement_client,
		transfer_id,
		BridgeAddress(initiator.clone()),
		BridgeAddress(MovementAddress(recipient)),
		amount,
		incoming_nonce,
	)
	.await
	.expect("Failed to complete bridge transfer");

	// Use timeout to wait for the next event
	let event_option =
		tokio::time::timeout(std::time::Duration::from_secs(30), mvt_monitoring.next())
			.await
			.expect("Timeout while waiting for the ETH Locked event");

	// Check if we received an event (Option) and handle the Result inside it
	let details = match event_option {
		Some(Ok(BridgeContractEvent::Completed(detail))) => detail,
		Some(Err(e)) => panic!("Error in bridge contract event: {:?}", e),
		None => panic!("No event received"),
		_ => panic!("Not a an Initiated event: {:?}", event_option),
	};

	assert_eq!(details.bridge_transfer_id.0, transfer_id.0);
	assert_eq!(details.initiator.0, initiator, "Initiator address does not match");
	assert_eq!(details.recipient.0, MovementAddress(recipient));
	assert_eq!(details.amount, amount);
	assert_eq!(details.nonce, incoming_nonce);

	Ok(())
}
