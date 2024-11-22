use alloy::primitives::keccak256;
use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::account_address::AccountAddress;
use bridge_integration_tests::utils as test_utils;
use bridge_integration_tests::{EthToMovementCallArgs, MovementToEthCallArgs, TestHarness};
use bridge_service::chains::bridge_contracts::BridgeContractEvent;
use bridge_service::chains::movement::event_monitoring::MovementMonitoring;
use bridge_service::{
	chains::{
		bridge_contracts::BridgeContract,
		movement::utils::{MovementAddress, MovementHash},
	},
	types::{Amount, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage},
};
use chrono::Utc;
use futures::StreamExt;
use tokio::time::{sleep, Duration};
use tokio::{self};
use tracing::info;

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
			let res = BridgeContract::initiate_bridge_transfer(
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
		let bridge_transfer_id = match event_option {
			Some(Ok(BridgeContractEvent::Initiated(detail))) => detail.bridge_transfer_id,
			Some(Err(e)) => panic!("Error in bridge contract event: {:?}", e),
			None => panic!("No event received"),
			_ => panic!("Not a an Initiated event: {:?}", event_option),
		};

		tracing::info!("Received bridge_transfer_id: {:?}", bridge_transfer_id);

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			&mut mvt_client_harness.movement_client,
			bridge_transfer_id,
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		test_utils::assert_initiator_bridge_transfer_details_framework(
			&details,
			details.bridge_transfer_id.0.to_vec(),
			details.initiator.to_string(),
			details.recipient.to_vec(),
			details.amount.0,
		);

		Ok(())
	}
	.await;

	test_result
}

#[tokio::test]
async fn test_movement_relayer_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut mvt_client_harness, _config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
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

	let mut padded_secret = [0u8; 32];
	padded_secret[..secret.len()].copy_from_slice(secret);

	BridgeRelayerContract::complete_bridge_transfer(
		&mut mvt_client_harness.movement_client,
		transfer_id,
		hash_lock_pre_image,
	)
	.await
	.expect("Failed to complete bridge transfer");

	let details = BridgeContract::get_bridge_transfer_details_counterparty(
		&mut mvt_client_harness.movement_client,
		transfer_id,
	)
	.await
	.expect("Failed to get bridge transfer details")
	.expect("Expected to find bridge transfer details, but got None");

	assert_eq!(details.bridge_transfer_id.0, transfer_id.0);
	assert_eq!(&details.initiator.0, &initiator, "Initiator address does not match");
	assert_eq!(details.recipient.0, MovementAddress(recipient));
	assert_eq!(details.amount.0, *amount);
	assert_eq!(details.nonce.0, *nonce);

	Ok(())
}
