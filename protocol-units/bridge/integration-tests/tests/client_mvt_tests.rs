use alloy::primitives::keccak256;
use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::account_address::AccountAddress;
use bridge_integration_tests::utils as test_utils;
use bridge_integration_tests::{EthToMovementCallArgs, MovementToEthCallArgs, TestHarness};
use bridge_service::{
	chains::{
		bridge_contracts::BridgeContract,
		movement::utils::{MovementAddress, MovementHash},
	},
	types::{Amount, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage},
};
use chrono::Utc;
use tokio::time::{sleep, Duration};
use tokio::{self};
use tracing::info;

#[tokio::test]
async fn test_movement_client_counterparty_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut mvt_client_harness, _config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let amount = Amount(1);
	let transfer_id = BridgeTransferId::gen_unique_hash(&mut rand::rngs::OsRng);
	let initiator = b"32Be343B94f860124dC4fEe278FDCBD38C102D88".to_vec();
	let recipient = AccountAddress::new(*b"0x00000000000000000000000000face");

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

	mvt_client_harness
		.movement_client
		.lock_bridge_transfer(
			transfer_id,
			hash_lock,
			BridgeAddress(initiator.clone()),
			BridgeAddress(MovementAddress(recipient.clone())),
			amount,
		)
		.await
		.expect("Failed to lock bridge transfer");

	let details = BridgeContract::get_bridge_transfer_details_counterparty(
		&mut mvt_client_harness.movement_client,
		transfer_id,
	)
	.await
	.expect("Failed to get bridge transfer details")
	.expect("Expected to find bridge transfer details, but got None");

	assert_eq!(details.state, 1, "Bridge transfer should be pending.");
	info!("Bridge transfer details: {:?}", details);

	let secret = b"secret";
	let mut padded_secret = [0u8; 32];
	padded_secret[..secret.len()].copy_from_slice(secret);

	BridgeContract::counterparty_complete_bridge_transfer(
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
	assert_eq!(details.hash_lock.0, hash_lock.0);
	assert_eq!(&details.initiator_address.0, &initiator, "Initiator address does not match");
	assert_eq!(details.recipient_address.0, MovementAddress(recipient));
	assert_eq!(details.amount.0, *amount);
	assert_eq!(details.state, 2, "Bridge transfer is supposed to be completed but it's not.");

	Ok(())
}

#[tokio::test]
async fn test_movement_client_initiate_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
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

#[tokio::test]
async fn test_movement_client_abort_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut mvt_client_harness, _config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let args = EthToMovementCallArgs::default();
	let coin_client = CoinClient::new(&mvt_client_harness.rest_client);
	let movement_client_signer = mvt_client_harness.movement_client.signer();

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
	}

	let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
	assert!(
		balance >= 100_000_000,
		"Expected Movement Client to have at least 100_000_000, but found {}",
		balance
	);

	mvt_client_harness
		.movement_client
		.lock_bridge_transfer(
			BridgeTransferId(args.bridge_transfer_id.0),
			HashLock(args.hash_lock.0),
			BridgeAddress(args.initiator.clone()),
			BridgeAddress(args.recipient.clone().into()),
			Amount(args.amount),
		)
		.await
		.expect("Failed to lock bridge transfer");

	let details = BridgeContract::get_bridge_transfer_details_counterparty(
		&mut mvt_client_harness.movement_client,
		BridgeTransferId(args.bridge_transfer_id.0),
	)
	.await
	.expect("Failed to get bridge transfer details")
	.expect("Expected to find bridge transfer details, but got None");

	assert_eq!(details.state, 1, "Bridge transfer should be pending.");

	sleep(Duration::from_secs(20)).await;

	let secret = b"secret";
	let mut padded_secret = [0u8; 32];
	padded_secret[..secret.len()].copy_from_slice(secret);

	BridgeContract::abort_bridge_transfer(
		&mut mvt_client_harness.movement_client,
		BridgeTransferId(args.bridge_transfer_id.0),
	)
	.await
	.expect("Failed to complete bridge transfer");

	let details = BridgeContract::get_bridge_transfer_details_counterparty(
		&mut mvt_client_harness.movement_client,
		BridgeTransferId(args.bridge_transfer_id.0),
	)
	.await
	.expect("Failed to get bridge transfer details")
	.expect("Expected to find bridge transfer details, but got None");

	assert_eq!(details.bridge_transfer_id.0, args.bridge_transfer_id.0);
	assert_eq!(details.hash_lock.0, args.hash_lock.0);
	assert_eq!(&details.initiator_address.0, &args.initiator, "Initiator address does not match");
	assert_eq!(details.recipient_address.0, args.recipient);
	assert_eq!(details.amount.0, args.amount);
	assert_eq!(details.state, 3, "Bridge transfer is supposed to be cancelled but it's not.");

	Ok(())
}

// Failing with EINVALID_PRE_IMAGE(0x1). Client and unit tests for modules used in client_movement_eth pass.
#[tokio::test]
async fn test_movement_client_initiator_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (mut mvt_client_harness, _config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let args = MovementToEthCallArgs::default();
	test_utils::fund_and_check_balance_framework(&mut mvt_client_harness, 100_000_000_000).await?;
	test_utils::initiate_bridge_transfer_helper_framework(
		&mut mvt_client_harness.movement_client,
		args.initiator.0,
		args.recipient.clone(),
		args.hash_lock.0,
		args.amount,
	)
	.await
	.expect("Failed to initiate bridge transfer");

	let bridge_transfer_id: [u8; 32] =
		test_utils::extract_bridge_transfer_id_framework(&mut mvt_client_harness.movement_client)
			.await?;
	info!("Bridge transfer ID: {:?}", bridge_transfer_id);

	// let details = BridgeContract::get_bridge_transfer_details_initiator(
	// 	&mut mvt_client_harness.movement_client,
	// 	BridgeTransferId(bridge_transfer_id),
	// )
	// .await
	// .expect("Failed to get bridge transfer details")
	// .expect("Expected to find bridge transfer details, but got None");

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

#[tokio::test]
async fn test_movement_client_refund_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();

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
