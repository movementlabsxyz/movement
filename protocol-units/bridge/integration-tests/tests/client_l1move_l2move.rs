use alloy::{
	primitives::{address, keccak256},
	providers::Provider,
};
use anyhow::Result;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::coin_client::CoinClient;
use aptos_types::PeerId;
use bridge_config::Config;
use bridge_integration_tests::utils as test_utils;
use bridge_integration_tests::EthToMovementCallArgs;
use bridge_integration_tests::HarnessMvtClient;
use bridge_integration_tests::{TestHarness, TestHarnessFramework};
use bridge_service::chains::{ethereum::types::EthAddress, movement::client_framework::MovementClientFramework};
use bridge_service::chains::{
	bridge_contracts::{BridgeContract, BridgeContractEvent}, ethereum::types::EthHash, movement::{event_monitoring::MovementMonitoring, utils::MovementHash}
};
use bridge_service::types::{
	Amount, AssetType, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage,
};
use futures::StreamExt;
use tokio::time::{sleep, Duration};
use tokio::{self};
use tracing::info;

#[tokio::test]
async fn test_movement_client_lock_transfer(
) -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	MovementClientFramework::bridge_setup_scripts().await?;
	let config: Config = Config::suzuka();
	let (mut mvt_client_harness, config) = TestHarnessFramework::new_with_suzuka(config).await;
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
				Amount(AssetType::Moveth(args.amount)),
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
async fn test_movement_client_complete_transfer(
) -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	MovementClientFramework::bridge_setup_scripts().await?;
	let config: Config = Config::suzuka();
	let (mut mvt_client_harness, config) = TestHarnessFramework::new_with_suzuka(config).await;
	let args = EthToMovementCallArgs::default();
	let test_result = async {
		let coin_client = CoinClient::new(&mvt_client_harness.rest_client);
		let movement_client_signer = mvt_client_harness.movement_client.signer();
		{
			let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
			faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
			faucet_client.fund(AccountAddress::from_hex_literal("0xface")?, 100_000_000).await?;
			faucet_client.fund(AccountAddress::from_hex_literal("0x1")?, 100_000_000).await?;
			faucet_client.fund(AccountAddress::from_hex_literal("0x3078303030303030303030303030303030303030303030303030303066616365")?, 100_000_000).await?;
			
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
				Amount(AssetType::Moveth(args.amount)),
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
			&details.initiator_address.0,
			&args.initiator,
			"Initiator address does not match"
		);
		assert_eq!(details.recipient_address.0, args.recipient);
		assert_eq!(details.amount.0, AssetType::Moveth(args.amount));
		assert_eq!(details.state, 2, "Bridge transfer is supposed to be completed but it's not.");

		Ok(())
	}
	.await;

	test_result
}

#[tokio::test]
async fn test_movement_client_abort_transfer(
) -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();
	MovementClientFramework::bridge_setup_scripts().await?;
	let config: Config = Config::suzuka();
	let (mut mvt_client_harness, config) = TestHarnessFramework::new_with_suzuka(config).await;
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
				Amount(AssetType::Moveth(args.amount)),
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
			BridgeTransferId(args.bridge_transfer_id.0)
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
			&details.initiator_address.0,
			&args.initiator,
			"Initiator address does not match"
		);
		assert_eq!(details.recipient_address.0, args.recipient);
		assert_eq!(details.amount.0, AssetType::Moveth(args.amount));
		assert_eq!(details.state, 3, "Bridge transfer is supposed to be cancelled but it's not.");

		Ok(())
	}
	.await;

	test_result
}
#[tokio::test]
async fn test_eth_client_should_build_and_fetch_accounts() {
	let config = Config::default();
	let (eth_client_harness, _config, _anvil) = TestHarness::new_only_eth(config).await;

	let expected_accounts = [
		address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266"),
		address!("70997970c51812dc3a010c7d01b50e0d17dc79c8"),
		address!("3c44cdddb6a900fa2b585dd299e03d12fa4293bc"),
		address!("90f79bf6eb2c4f870365e785982e1f101e93b906"),
		address!("15d34aaf54267db7d7c367839aaf71a00a2c6a65"),
		address!("9965507d1a55bcc2695c58ba16fb37d819b0a4dc"),
		address!("976ea74026e726554db657fa54763abd0c3a0aa9"),
		address!("14dc79964da2c08b23698b3d3cc7ca32193d9955"),
		address!("23618e81e3f5cdf7f54c3d65f7fbc0abf5b21e8f"),
		address!("a0ee7a142d267c1f36714e4a8f75612f20a79720"),
	];

	let provider = eth_client_harness.rpc_provider().await;
	let accounts = provider.get_accounts().await.expect("Failed to get accounts");
	assert_eq!(accounts.len(), expected_accounts.len());

	for (account, expected) in accounts.iter().zip(expected_accounts.iter()) {
		assert_eq!(account, expected);
	}
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
			Amount(AssetType::EthAndWeth((1, 0))), // Eth
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
	eth_client_harness
		.deposit_weth_and_approve(
			BridgeAddress(EthAddress(signer_address)),
			Amount(AssetType::EthAndWeth((0, 1))),
		)
		.await
		.expect("Failed to deposit WETH");

	eth_client_harness
		.eth_client
		.initiate_bridge_transfer(
			BridgeAddress(EthAddress(signer_address)),
			BridgeAddress(recipient),
			HashLock(EthHash(hash_lock).0),
			Amount(AssetType::EthAndWeth((0, 1))),
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
	eth_client_harness
		.deposit_weth_and_approve(
			BridgeAddress(EthAddress(signer_address)),
			Amount(AssetType::EthAndWeth((0, 1))),
		)
		.await
		.expect("Failed to deposit WETH");

	eth_client_harness
		.eth_client
		.initiate_bridge_transfer(
			BridgeAddress(EthAddress(signer_address)),
			BridgeAddress(recipient),
			HashLock(EthHash(hash_lock).0),
			Amount(AssetType::EthAndWeth((1, 1))),
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
			Amount(AssetType::EthAndWeth((1000, 0))), // Eth
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
			Amount(AssetType::EthAndWeth((42, 0))),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	//TODO: Here call complete with the id captured from the event
}
