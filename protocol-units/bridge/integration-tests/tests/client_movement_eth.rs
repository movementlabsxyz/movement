use anyhow::Result;
use bridge_config::Config;
use bridge_integration_tests::utils;
use bridge_integration_tests::utils as test_utils;
use bridge_integration_tests::{MovementToEthCallArgs, TestHarness};
use bridge_service::{
	chains::{bridge_contracts::BridgeContract, movement::utils::MovementHash},
	types::{BridgeTransferId, HashLockPreImage},
};
use tokio::time::{sleep, Duration};
use tokio::{self};
use tracing::info;

#[tokio::test]
async fn test_movement_client_build_and_fund_accounts() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let config = Config::default();
	let (mut mvt_client_harness, _config, mut mvt_process) =
		TestHarness::new_with_movement(config).await;
	let test_result = async {
		test_utils::fund_and_check_balance(&mut mvt_client_harness, 100_000_000_000)
			.await
			.expect("Failed to fund accounts");
		Ok(())
	}
	.await;

	if let Err(e) = mvt_process.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}
	test_result
}

#[tokio::test]
async fn test_movement_client_initiate_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let config = Config::default();
	let (mut mvt_client_harness, _config, mut mvt_process) =
		TestHarness::new_with_movement(config).await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let sender_address = mvt_client_harness.movement_client.signer().address();
		test_utils::fund_and_check_balance(&mut mvt_client_harness, 100_000_000_000).await?;
		test_utils::initiate_bridge_transfer_helper(
			&mut mvt_client_harness.movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
			true,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(&mut mvt_client_harness.movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		test_utils::assert_bridge_transfer_details(
			&details,
			MovementHash(bridge_transfer_id).0,
			MovementHash(args.hash_lock.0).0,
			sender_address,
			args.recipient.clone(),
			args.amount,
			1,
		);

		Ok(())
	}
	.await;

	if let Err(e) = mvt_process.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}

#[tokio::test]
#[ignore] // Test fail when run with the other test: https://github.com/movementlabsxyz/movement/issues/656
async fn test_movement_client_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let default_config = Config::default();
	let (mut mvt_client_harness, _config, mut mvt_process) =
		TestHarness::new_with_movement(default_config).await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let sender_address = mvt_client_harness.movement_client.signer().address();
		test_utils::fund_and_check_balance(&mut mvt_client_harness, 100_000_000_000).await?;
		test_utils::initiate_bridge_transfer_helper(
			&mut mvt_client_harness.movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
			true,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(&mut mvt_client_harness.movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		test_utils::assert_bridge_transfer_details(
			&details,
			MovementHash(bridge_transfer_id).0,
			MovementHash(args.hash_lock.0).0,
			sender_address,
			args.recipient.clone(),
			args.amount,
			1,
		);

		let secret = b"secret";
		let mut padded_secret = [0u8; 32];
		padded_secret[..secret.len()].copy_from_slice(secret);

		BridgeContract::initiator_complete_bridge_transfer(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
			HashLockPreImage(padded_secret),
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.state, 2, "Bridge transfer should be completed.");

		Ok(())
	}
	.await;

	if let Err(e) = mvt_process.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}

#[tokio::test]
async fn test_movement_client_refund_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let config = Config::default();
	let (mut mvt_client_harness, _config, mut mvt_process) =
		TestHarness::new_with_movement(config).await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let sender_address = mvt_client_harness.movement_client.signer().address();
		test_utils::fund_and_check_balance(&mut mvt_client_harness, 100_000_000_000).await?;

		let ledger_info = mvt_client_harness.rest_client.get_ledger_information().await?;
		println!("Ledger info: {:?}", ledger_info);

		test_utils::initiate_bridge_transfer_helper(
			&mut mvt_client_harness.movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
			true,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(&mut mvt_client_harness.movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		utils::assert_bridge_transfer_details(
			&details,
			MovementHash(bridge_transfer_id).0,
			MovementHash(args.hash_lock.0).0,
			sender_address,
			args.recipient.clone(),
			args.amount,
			1,
		);

		sleep(Duration::from_secs(2)).await;

		BridgeContract::refund_bridge_transfer(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to complete bridge transfer");

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

	if let Err(e) = mvt_process.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}
