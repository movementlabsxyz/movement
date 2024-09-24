use tokio::time::{sleep, Duration}; // Add these imports

use anyhow::Result;
use bridge_integration_tests::TestHarness;
use bridge_integration_tests::{
	utils::{self as test_utils},
	MovementToEthCallArgs,
};
use bridge_shared::{
	bridge_contracts::BridgeContractInitiator,
	types::{BridgeTransferId, HashLockPreImage},
};
use tokio::{self};
use tracing::info;

#[tokio::test]
async fn test_movement_client_build_and_fund_accounts() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	let (mut harness, mut child) = TestHarness::new_with_movement().await;
	let test_result = async {
		let mut movement_client =
			harness.movement_client_mut().expect("Failed to get MovementClient");
		test_utils::fund_and_check_balance(&mut movement_client, 100_000_000_000)
			.await
			.expect("Failed to fund accounts");
		Ok(())
	}
	.await;

	if let Err(e) = child.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}
	test_result
}

#[tokio::test]
async fn test_movement_client_initiate_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let mut movement_client =
			harness.movement_client_mut().expect("Failed to get MovementClient");
		let sender_address = movement_client.signer().address();
		test_utils::fund_and_check_balance(&mut movement_client, 100_000_000_000).await?;
		test_utils::initiate_bridge_transfer_helper(
			&mut movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock,
			args.amount,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(&mut movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		test_utils::assert_bridge_transfer_details::<[u8; 32]>(
			&details,
			bridge_transfer_id,
			args.hash_lock,
			sender_address,
			args.recipient.clone(),
			args.amount,
			1,
		);

		Ok(())
	}
	.await;

	if let Err(e) = child.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}

#[tokio::test]
async fn test_movement_client_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let mut movement_client =
			harness.movement_client_mut().expect("Failed to get MovementClient");
		let sender_address = movement_client.signer().address();
		test_utils::fund_and_check_balance(&mut movement_client, 100_000_000_000).await?;
		test_utils::initiate_bridge_transfer_helper(
			&mut movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock,
			args.amount,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(&mut movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		test_utils::assert_bridge_transfer_details::<[u8; 32]>(
			&details,
			bridge_transfer_id,
			args.hash_lock,
			sender_address,
			args.recipient.clone(),
			args.amount,
			1,
		);

		BridgeContractInitiator::complete_bridge_transfer(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
			HashLockPreImage(b"secret".to_vec()),
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.state, 2, "Bridge transfer should be completed.");

		Ok(())
	}
	.await;

	if let Err(e) = child.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}

#[tokio::test]
async fn test_movement_client_refund_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;

	let args = MovementToEthCallArgs::default();

	let test_result = async {
		let mut movement_client =
			harness.movement_client_mut().expect("Failed to get MovementClient");
		let sender_address = movement_client.signer().address();
		test_utils::fund_and_check_balance(&mut movement_client, 100_000_000_000).await?;

		let ledger_info = movement_client.rest_client().get_ledger_information().await?;
		println!("Ledger info: {:?}", ledger_info);

		let active_timelock = movement_client.initiator_time_lock_duration().await?;
		println!("Active timelock: {:?}", active_timelock);

		// Set the timelock to 1 second for testing
		movement_client.initiator_set_timelock(1).await.expect("Failed to set timelock");

		let active_timelock = movement_client.initiator_time_lock_duration().await?;
		println!("Active timelock: {:?}", active_timelock);

		test_utils::initiate_bridge_transfer_helper(
			&mut movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock,
			args.amount,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(&mut movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		test_utils::assert_bridge_transfer_details::<[u8; 32]>(
			&details,
			bridge_transfer_id,
			args.hash_lock,
			sender_address,
			args.recipient.clone(),
			args.amount,
			1,
		);

		sleep(Duration::from_secs(2)).await;

		BridgeContractInitiator::refund_bridge_transfer(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContractInitiator::get_bridge_transfer_details(
			movement_client,
			BridgeTransferId(bridge_transfer_id),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.state, 3, "Bridge transfer should be refunded.");

		Ok(())
	}
	.await;

	if let Err(e) = child.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}
