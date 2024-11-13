use alloy::primitives::keccak256;
use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::account_address::AccountAddress;
use bridge_integration_tests::utils as test_utils;
use bridge_integration_tests::EthToMovementCallArgs;
use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::HarnessMvtClient;
use bridge_integration_tests::TestHarness;
use bridge_service::chains::bridge_contracts::BridgeContract;
use bridge_service::chains::ethereum::types::EthAddress;
use bridge_service::types::{Amount, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage};
use tokio::time::{sleep, Duration};
use tokio::{self};
use tracing::info;

#[tokio::test]
async fn test_movement_client_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	let (mut mvt_client_harness, _config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let args = EthToMovementCallArgs::default();
	let test_result = async {
		let coin_client = CoinClient::new(&mvt_client_harness.rest_client);
		let movement_client_signer = mvt_client_harness.movement_client.signer();
		{
			let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
			faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
			faucet_client
				.fund(AccountAddress::from_hex_literal("0xface")?, 100_000_000)
				.await?;
			faucet_client
				.fund(AccountAddress::from_hex_literal("0x1")?, 100_000_000)
				.await?;
			// This address is the recipient in test_movement_client_complete_transfer, so it needs an AptosCoin store
			faucet_client
				.fund(
					AccountAddress::from_hex_literal(
						"0x3078303030303030303030303030303030303030303030303030303066616365",
					)?,
					100_000_000,
				)
				.await?;
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

		info!("Recipient: {:?}", details.recipient_address);

		assert_eq!(details.state, 1, "Bridge transfer should be pending.");
		info!("Bridge transfer details: {:?}", details);

		let secret = b"secret";
		let mut padded_secret = [0u8; 32];
		padded_secret[..secret.len()].copy_from_slice(secret);

		BridgeContract::counterparty_complete_bridge_transfer(
			&mut mvt_client_harness.movement_client,
			BridgeTransferId(args.bridge_transfer_id.0),
			HashLockPreImage(padded_secret),
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
		assert_eq!(
			&details.initiator_address.0, &args.initiator,
			"Initiator address does not match"
		);
		assert_eq!(details.recipient_address.0, args.recipient);
		assert_eq!(details.amount.0, args.amount);
		assert_eq!(details.state, 2, "Bridge transfer is supposed to be completed but it's not.");

		Ok(())
	}
	.await;

	test_result
}

#[tokio::test]
async fn test_eth_client_complete_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let config = TestHarness::read_bridge_config().await.unwrap();
	let (mut eth_client_harness, _config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	// initialize Eth transfer
	tracing::info!("Call initiate_transfer on Eth");
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let amount = Amount(1);

	let transfer_id = BridgeTransferId::gen_unique_hash(&mut rand::rngs::OsRng);
	let res = BridgeContract::lock_bridge_transfer(
		&mut eth_client_harness.eth_client,
		transfer_id,
		hash_lock,
		BridgeAddress(vec![3; 32]),
		BridgeAddress(EthAddress(HarnessEthClient::get_recipeint_address(&config))),
		amount,
	)
	.await;
	println!("lock res{res:?}",);
	assert!(res.is_ok());

	tracing::info!("Bridge transfer ID from Eth Lock event: {:?}", transfer_id);

	BridgeContract::counterparty_complete_bridge_transfer(
		&mut eth_client_harness.eth_client,
		transfer_id,
		hash_lock_pre_image,
	)
	.await
	.expect("Failed to complete bridge transfer");
}

#[tokio::test]
async fn test_movement_client_abort_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	let (mut mvt_client_harness, _config) =
		TestHarness::new_with_movement().await.expect("Bridge config file not set");
	let args = EthToMovementCallArgs::default();
	let test_result = async {
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

		info!("Bridge transfer details: {:?}", details);

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
		assert_eq!(
			&details.initiator_address.0, &args.initiator,
			"Initiator address does not match"
		);
		assert_eq!(details.recipient_address.0, args.recipient);
		assert_eq!(details.amount.0, args.amount);
		assert_eq!(details.state, 3, "Bridge transfer is supposed to be cancelled but it's not.");

		Ok(())
	}
	.await;

	test_result
}

#[tokio::test]
async fn test_eth_client_lock_transfer() {
	let config = TestHarness::read_bridge_config().await.unwrap();
	let (mut eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	// Call lock transfer Eth
	tracing::info!("Call lockBridgeTransfer on Eth");
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let amount = Amount(1);
	let transfer_id = BridgeTransferId::gen_unique_hash(&mut rand::rngs::OsRng);

	let res = eth_client_harness
		.eth_client
		.lock_bridge_transfer(
			transfer_id,
			hash_lock,
			BridgeAddress(vec![3; 32]),
			BridgeAddress(EthAddress(HarnessEthClient::get_recipeint_address(&config))),
			amount,
		)
		.await;

	assert!(res.is_ok(), "lock_bridge_transfer failed because: {res:?}");
}

#[tokio::test]
async fn test_eth_client_initiate_transfer() {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).try_init();
	let (_eth_client_harness, config) =
		TestHarness::new_only_eth().await.expect("Bridge config file not set");

	let recipient = HarnessMvtClient::gen_aptos_account();
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));

	let res = HarnessEthClient::initiate_eth_bridge_transfer(
		&config,
		HarnessEthClient::get_initiator_private_key(&config),
		bridge_service::chains::movement::utils::MovementAddress(recipient.address()),
		hash_lock,
		Amount(1),
	)
	.await;
	assert!(res.is_ok(), "initiate_bridge_transfer failed because: {res:?}");
}
