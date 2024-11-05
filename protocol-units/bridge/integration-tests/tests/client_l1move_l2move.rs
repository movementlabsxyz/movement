use alloy::primitives::{address, keccak256};
use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::account_address::AccountAddress;
use bridge_config::Config;
use bridge_integration_tests::EthToMovementCallArgs;
use bridge_integration_tests::HarnessMvtClient;
use bridge_integration_tests::{TestHarness, TestHarnessFramework};
use bridge_service::chains::{bridge_contracts::BridgeContract, ethereum::types::EthHash};
use bridge_service::chains::{
	ethereum::types::EthAddress, movement::client_framework::MovementClientFramework,
};
use bridge_service::types::{Amount, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage};
use tokio::time::{sleep, Duration};
use tokio::{self};
use tracing::info;

#[tokio::test]
async fn test_movement_client_lock_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	MovementClientFramework::bridge_setup_scripts().await?;
	let config: Config = Config::suzuka();
	let (mut mvt_client_harness, _config) = TestHarnessFramework::new_with_suzuka(config).await;
	let args = EthToMovementCallArgs::default();
	info! {"Args Initiator: {:?}", args.initiator};
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

		assert_eq!(details.state, 1, "Bridge transfer should be pending.");
		info!("Bridge transfer details: {:?}", details);
		Ok(())
	}
	.await;
	test_result
}

#[tokio::test]
async fn test_movement_client_complete_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	MovementClientFramework::bridge_setup_scripts().await?;
	let config: Config = Config::suzuka();
	let (mut mvt_client_harness, _config) = TestHarnessFramework::new_with_suzuka(config).await;
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
async fn test_movement_client_abort_transfer() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	MovementClientFramework::bridge_setup_scripts().await?;
	let config: Config = Config::suzuka();
	let (mut mvt_client_harness, _config) = TestHarnessFramework::new_with_suzuka(config).await;
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
async fn test_eth_client_should_deploy_initiator_contract() {
	let config = Config::default();
	let (_eth_client_harness, config, _anvil) = TestHarness::new_only_eth(config).await;

	assert!(config.eth.eth_initiator_contract != "Oxeee");
	assert_eq!(
		config.eth.eth_initiator_contract, "0x8464135c8F25Da09e49BC8782676a84730C318bC",
		"Wrong initiator contract address."
	);
}

#[tokio::test]
async fn test_eth_client_should_successfully_call_initialize() {
	let config = Config::default();
	let (_eth_client_harness, config, _anvil) = TestHarness::new_only_eth(config).await;
	assert!(config.eth.eth_counterparty_contract != "0xccc");
	assert_eq!(
		config.eth.eth_counterparty_contract, "0x71C95911E9a5D330f4D621842EC243EE1343292e",
		"Wrong initiator contract address."
	);
	assert!(config.eth.eth_weth_contract != "0xe3e3");
	assert_eq!(
		config.eth.eth_weth_contract, "0x948B3c65b89DF0B4894ABE91E6D02FE579834F8F",
		"Wrong initiator contract address."
	);
}

#[tokio::test]
async fn test_eth_client_should_successfully_call_initiate_transfer_only_eth() {
	let config = Config::default();
	let (mut eth_client_harness, _config, _anvil) = TestHarness::new_only_eth(config).await;

	let signer_address: alloy::primitives::Address = eth_client_harness.signer_address();

	let recipient = HarnessMvtClient::gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();
	eth_client_harness
		.eth_client
		.initiate_bridge_transfer(
			BridgeAddress(EthAddress(signer_address)),
			BridgeAddress(recipient),
			HashLock(EthHash(hash_lock).0),
			Amount(1000),
		)
		.await
		.expect("Failed to initiate bridge transfer");
}

#[tokio::test]
async fn test_eth_client_should_successfully_call_initiate_transfer_only_weth() {
	let config = Config::default();
	let (mut eth_client_harness, _config, _anvil) = TestHarness::new_only_eth(config).await;

	let signer_address: alloy::primitives::Address = eth_client_harness.signer_address();

	let recipient = HarnessMvtClient::gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();
	// eth_client_harness
	// 	.deposit_weth_and_approve(
	// 		BridgeAddress(EthAddress(signer_address)),
	// 		Amount(AssetType::EthAndWeth((0, 1))),
	// 	)
	// 	.await
	// 	.expect("Failed to deposit WETH");

	eth_client_harness
		.eth_client
		.initiate_bridge_transfer(
			BridgeAddress(EthAddress(signer_address)),
			BridgeAddress(recipient),
			HashLock(EthHash(hash_lock).0),
			Amount(1000),
		)
		.await
		.expect("Failed to initiate bridge transfer");
}

#[tokio::test]
async fn test_eth_client_should_successfully_call_initiate_transfer_eth_and_weth() {
	let config = Config::default();
	let (mut eth_client_harness, _config, _anvil) = TestHarness::new_only_eth(config).await;

	let signer_address: alloy::primitives::Address = eth_client_harness.signer_address();

	let recipient = HarnessMvtClient::gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();
	// eth_client_harness
	// 	.deposit_weth_and_approve(
	// 		BridgeAddress(EthAddress(signer_address)),
	// 		Amount(AssetType::EthAndWeth((0, 1))),
	// 	)
	// 	.await
	// 	.expect("Failed to deposit WETH");

	eth_client_harness
		.eth_client
		.initiate_bridge_transfer(
			BridgeAddress(EthAddress(signer_address)),
			BridgeAddress(recipient),
			HashLock(EthHash(hash_lock).0),
			Amount(1000),
		)
		.await
		.expect("Failed to initiate bridge transfer");
}

#[tokio::test]
#[ignore] // To be tested after this is merged in https://github.com/movementlabsxyz/movement/pull/209
async fn test_client_should_successfully_get_bridge_transfer_id() {
	let config = Config::default();
	let (mut eth_client_harness, _config, _anvil) = TestHarness::new_only_eth(config).await;

	let signer_address: alloy::primitives::Address = eth_client_harness.signer_address();

	let recipient = HarnessMvtClient::gen_aptos_account();
	let hash_lock: [u8; 32] = keccak256("secret".to_string().as_bytes()).into();

	eth_client_harness
		.eth_client
		.initiate_bridge_transfer(
			BridgeAddress(EthAddress(signer_address)),
			BridgeAddress(recipient),
			HashLock(EthHash(hash_lock).0),
			Amount(1000),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//TODO: Here call get details with the captured event
}

#[tokio::test]
#[ignore] // To be tested after this is merged in https://github.com/movementlabsxyz/movement/pull/209
async fn test_eth_client_should_successfully_complete_transfer() {
	let config = Config::default();
	let (mut eth_client_harness, _config, _anvil) = TestHarness::new_only_eth(config).await;

	let signer_address: alloy::primitives::Address = eth_client_harness.signer_address();

	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
	let recipient_bytes: Vec<u8> = recipient.to_string().as_bytes().to_vec();

	let secret = "secret".to_string();
	let hash_lock = keccak256(secret.as_bytes());
	let hash_lock: [u8; 32] = hash_lock.into();

	eth_client_harness
		.eth_client
		.initiate_bridge_transfer(
			BridgeAddress(EthAddress(signer_address)),
			BridgeAddress(recipient_bytes),
			HashLock(EthHash(hash_lock).0),
			Amount(1000),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//TODO: Here call complete with the id captured from the event
}
