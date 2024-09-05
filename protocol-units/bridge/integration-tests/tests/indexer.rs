use bridge_integration_tests::TestHarness;
use bridge_shared::bridge_contracts::BridgeContractCounterparty;
use bridge_shared::types::{
	Amount, AssetType, BridgeTransferId, HashLock, InitiatorAddress, RecipientAddress, TimeLock,
};

#[tokio::test]
async fn test_harness_should_start_indexer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (harness, mut harness_child) = TestHarness::new_with_movement().await;
	let mut _indexer_child = harness.start_indexer("127.0.0.1", 5432).await;

	// Wait for the indexer process to complete
	tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
	harness_child.kill().await.expect("Failed to kill the child process");
	Ok(())
}

#[tokio::test]
async fn test_indexer_should_capture_event_for_lock_call() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut _harness_child) = TestHarness::new_with_movement().await;
	harness.start_indexer("127.0.0.1", 5432).await;
	tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;

	{
		let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");
		let _ = movement_client.publish_for_test();
	}

	let args = harness.move_call_args();
	harness
		.movement_client_mut()
		.expect("Failed to get MovmentClient")
		.lock_bridge_transfer(
			BridgeTransferId(args.bridge_transfer_id),
			HashLock(args.hash_lock),
			TimeLock(args.time_lock),
			InitiatorAddress(args.initiator),
			RecipientAddress(args.recipient),
			Amount(AssetType::Moveth(args.amount)),
		)
		.await
		.expect("Failed to complete bridge transfer");

	Ok(())
}
