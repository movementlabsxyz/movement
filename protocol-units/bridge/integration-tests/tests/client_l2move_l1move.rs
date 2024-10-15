use alloy::primitives::keccak256;
use anyhow::Result;
use bridge_config::Config;
use bridge_integration_tests::utils;
use bridge_integration_tests::utils as test_utils;
use bridge_integration_tests::{MovementToEthCallArgs, TestHarness, TestHarnessFramework};
use bridge_service::chains::bridge_contracts::{BridgeContractError, BridgeContractEvent};
use bridge_service::chains::movement::client_framework::MovementClientFramework;
use bridge_service::chains::movement::event_monitoring_framework::MovementMonitoringFramework;
use bridge_service::types::AssetType;
use bridge_service::{
	chains::{
		bridge_contracts::BridgeContract,
		movement::{event_monitoring::MovementMonitoring, utils::MovementHash},
	},
	types::{BridgeTransferId, HashLockPreImage},
};
use chrono::Utc;
use futures::StreamExt;
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

	MovementClientFramework::bridge_setup_scripts().await?;

	let config: Config = Config::suzuka();

	let (mut mvt_client_harness, config) = TestHarnessFramework::new_with_suzuka(config).await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let sender_address = mvt_client_harness.movement_client.signer().address();
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
		
		let amount = match details.amount.0 {
			AssetType::Moveth(amount) => amount,
			_ => panic!("Expected Moveth asset type but found something else"),
		};

		test_utils::assert_counterparty_bridge_transfer_details_framework(
			&details,
			details.initiator_address.to_string(),
			details.recipient_address.to_vec(),
			amount,
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

	MovementClientFramework::bridge_setup_scripts().await?;

	let config: Config = Config::suzuka();

	let (mut mvt_client_harness, config) = TestHarnessFramework::new_with_suzuka(config).await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let sender_address = mvt_client_harness.movement_client.signer().address();
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

		assert_eq!(details.state, 1, "Bridge transfer should be initiated.");
		
		let amount = match details.amount.0 {
			AssetType::Moveth(amount) => amount,
			_ => panic!("Expected Moveth asset type but found something else"),
		};
		test_utils::assert_counterparty_bridge_transfer_details_framework(
			&details,
			details.initiator_address.to_string(),
			details.recipient_address.to_vec(),
			amount,
			details.hash_lock.0,
			details.time_lock.0,
		);

		assert_eq!(details.state, 2, "Bridge transfer should be completed.");

		Ok(())
	}
	.await;

	test_result
}

#[tokio::test]
async fn test_movement_client_refund_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	MovementClientFramework::bridge_setup_scripts().await?;

	let config: Config = Config::suzuka();

	let (mut mvt_client_harness, config) = TestHarnessFramework::new_with_suzuka(config).await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let sender_address = mvt_client_harness.movement_client.signer().address();
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


