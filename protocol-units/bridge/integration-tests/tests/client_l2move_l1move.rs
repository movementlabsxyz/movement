use anyhow::Result;
use bridge_integration_tests::utils as test_utils;
use bridge_integration_tests::{MovementToEthCallArgs, TestHarness};
use bridge_service::{
	chains::{bridge_contracts::BridgeContract, movement::utils::MovementHash},
	types::{BridgeTransferId, HashLockPreImage},
};
use chrono::Utc;
use tokio::time::{sleep, Duration};
use tokio::{self};
use tracing::info;

#[tokio::test]
async fn test_movement_client_initiate_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	let (mut mvt_client_harness, _config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let args = MovementToEthCallArgs::default();

	let test_result = async {
		test_utils::fund_and_check_balance_framework(&mut mvt_client_harness, 100_000_000_000)
			.await?;
		test_utils::initiate_bridge_transfer_helper_framework(
			&mut mvt_client_harness.movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] = test_utils::extract_bridge_transfer_id_framework(
			&mut mvt_client_harness.movement_client,
		)
		.await?;
		info!("Bridge transfer ID: {:?}", bridge_transfer_id);

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		info!("Bridge transfer details: {:?}", details);

		assert_eq!(details.state, 1, "Bridge transfer should be initiated.");

		test_utils::assert_counterparty_bridge_transfer_details_framework(
			&details,
			details.initiator_address.to_string(),
			details.recipient_address.to_vec(),
			details.amount.0,
			details.hash_lock.0,
			details.time_lock.0,
		);

		Ok(())
	}
	.await;

	test_result
}

// Failing with EINVALID_PRE_IMAGE(0x1). Client and unit tests for modules used in client_movement_eth pass.
#[tokio::test]
async fn test_movement_client_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	let (mut mvt_client_harness, config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let args = MovementToEthCallArgs::default();
	async {
		test_utils::fund_and_check_balance_framework(&mut mvt_client_harness, 100_000_000_000)
			.await?;
		test_utils::initiate_bridge_transfer_helper_framework(
			&mut mvt_client_harness.movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] = test_utils::extract_bridge_transfer_id_framework(
			&mut mvt_client_harness.movement_client,
		)
		.await?;
		info!("Bridge transfer ID: {:?}", bridge_transfer_id);

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		info!("Bridge transfer details: {:?}", details);

		let secret = b"secret";
		let mut padded_secret = [0u8; 32];
		padded_secret[..secret.len()].copy_from_slice(secret);

		BridgeContract::initiator_complete_bridge_transfer(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(bridge_transfer_id),
			HashLockPreImage(padded_secret),
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		info!("Bridge transfer details: {:?}", details);

		test_utils::assert_counterparty_bridge_transfer_details_framework(
			&details,
			details.initiator_address.to_string(),
			details.recipient_address.to_vec(),
			details.amount.0,
			details.hash_lock.0,
			details.time_lock.0,
		);

		assert_eq!(details.state, 2, "Bridge transfer should be completed.");

		Ok(())
	}
	.await
}

#[tokio::test]
async fn test_movement_client_refund_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut mvt_client_harness, _config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let args = MovementToEthCallArgs::default();

	let test_result = async {
		test_utils::fund_and_check_balance_framework(&mut mvt_client_harness, 100_000_000_000)
			.await?;
		test_utils::initiate_bridge_transfer_helper_framework(
			&mut mvt_client_harness.movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] = test_utils::extract_bridge_transfer_id_framework(
			&mut mvt_client_harness.movement_client,
		)
		.await?;
		info!("Bridge transfer ID: {:?}", bridge_transfer_id);

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		info!("Time lock: {:?}", details.time_lock);

		sleep(Duration::from_secs(20)).await;

		info!("Current timestamp: {:?}", Utc::now().timestamp());

		BridgeContract::refund_bridge_transfer(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to refund bridge transfer");

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.state, 3, "Bridge transfer should be refunded.");

		Ok(())
	}
	.await;

	test_result
}
