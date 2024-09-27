use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use bridge_service::{
	chains::{bridge_contracts::BridgeContract, movement::utils::MovementHash},
	types::{Amount, AssetType, BridgeAddress, BridgeTransferId, HashLock, HashLockPreImage},
};
use harness::{EthToMovementCallArgs, MovementToEthCallArgs, TestHarness};
use tokio::time::{sleep, Duration};
use tokio::{self};
use tracing::info;
mod utils;
use utils as test_utils;

mod harness;

#[tokio::test]
async fn test_movement_client_build_and_fund_accounts() -> Result<(), anyhow::Error> {
	let (scaffold, mut child) = TestHarness::new_with_movement().await;
	let movement_client = scaffold.movement_client().expect("Failed to get MovementClient");
	//
	let rest_client = movement_client.rest_client();
	let coin_client = CoinClient::new(&rest_client);
	let faucet_client = movement_client.faucet_client().expect("Failed to get // FaucetClient");
	let movement_client_signer = movement_client.signer();

	let faucet_client = faucet_client.write().unwrap();

	faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
	let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
	assert!(
		balance >= 100_000_000,
		"Expected Movement Client to have at least 100_000_000, but found {}",
		balance
	);

	child.kill().await?;

	Ok(())
}

#[tokio::test]
async fn test_movement_client_should_publish_package() -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;
	{
		let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");

		movement_client.publish_for_test()?;
	}

	child.kill().await?;

	Ok(())
}

#[tokio::test]

async fn test_movement_client_should_successfully_call_lock_and_complete(
) -> Result<(), anyhow::Error> {
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;

	let args = EthToMovementCallArgs::default();

	let test_result = async {
		let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");
		movement_client.publish_for_test()?;

		let rest_client = movement_client.rest_client();
		let coin_client = CoinClient::new(&rest_client);
		let faucet_client = movement_client.faucet_client().expect("Failed to get FaucetClient");
		let movement_client_signer = movement_client.signer();

		{
			let faucet_client = faucet_client.write().unwrap();
			faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
		}

		let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
		assert!(
			balance >= 100_000_000,
			"Expected Movement Client to have at least 100_000_000, but found {}",
			balance
		);

		movement_client
			.lock_bridge_transfer(
				BridgeTransferId(args.bridge_transfer_id.0),
				HashLock(args.hash_lock.0),
				BridgeAddress(args.initiator.clone()),
				BridgeAddress(args.recipient.clone().into()),
				Amount(AssetType::Moveth(args.amount)),
			)
			.await
			.expect("Failed to lock bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContract::get_bridge_transfer_details_counterparty(
			movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.bridge_transfer_id.0, args.bridge_transfer_id.0);
		assert_eq!(details.hash_lock.0, args.hash_lock.0);
		assert_eq!(
			&details.initiator_address.0 .0[32 - args.initiator.len()..],
			&args.initiator,
			"Initiator address does not match"
		);
		assert_eq!(details.recipient_address.0, args.recipient.0.to_vec());
		assert_eq!(details.amount.0, AssetType::Moveth(args.amount));
		assert_eq!(details.state, 1, "Bridge transfer is supposed to be locked but it's not.");

		let secret = b"secret";
		let mut padded_secret = [0u8; 32];
		padded_secret[..secret.len()].copy_from_slice(secret);

		BridgeContract::counterparty_complete_bridge_transfer(
			movement_client,
			BridgeTransferId(args.bridge_transfer_id.0),
			HashLockPreImage(padded_secret),
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContract::get_bridge_transfer_details_counterparty(
			movement_client,
			BridgeTransferId(args.bridge_transfer_id.0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.bridge_transfer_id.0, args.bridge_transfer_id.0);
		assert_eq!(details.hash_lock.0, args.hash_lock.0);
		assert_eq!(
			&details.initiator_address.0 .0[32 - args.initiator.len()..],
			&args.initiator,
			"Initiator address does not match"
		);
		assert_eq!(details.recipient_address.0, args.recipient.0.to_vec());
		assert_eq!(details.amount.0, AssetType::Moveth(args.amount));
		assert_eq!(details.state, 2, "Bridge transfer is supposed to be completed but it's not.");

		Ok(())
	}
	.await;

	if let Err(e) = child.kill().await {
		eprintln!("Failed to kill child process: {:?}", e);
	}

	test_result
}

#[tokio::test]
async fn test_movement_client_should_successfully_call_lock_and_abort() -> Result<(), anyhow::Error>
{
	let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).try_init();

	let (mut harness, mut child) = TestHarness::new_with_movement().await;

	let args = EthToMovementCallArgs::default();

	let test_result = async {
		let movement_client = harness.movement_client_mut().expect("Failed to get MovementClient");
		movement_client.publish_for_test()?;

		let rest_client = movement_client.rest_client();
		let coin_client = CoinClient::new(&rest_client);
		let faucet_client = movement_client.faucet_client().expect("Failed to get FaucetClient");
		let movement_client_signer = movement_client.signer();

		{
			let faucet_client = faucet_client.write().unwrap();
			faucet_client.fund(movement_client_signer.address(), 100_000_000).await?;
		}

		let balance = coin_client.get_account_balance(&movement_client_signer.address()).await?;
		assert!(
			balance >= 100_000_000,
			"Expected Movement Client to have at least 100_000_000, but found {}",
			balance
		);

		// Set the timelock to 1 second for testing
		movement_client
			.counterparty_set_timelock(1)
			.await
			.expect("Failed to set timelock");

		movement_client
			.lock_bridge_transfer(
				BridgeTransferId(args.bridge_transfer_id.0),
				HashLock(args.hash_lock.0),
				BridgeAddress(args.initiator.clone()),
				BridgeAddress(args.recipient.clone()),
				Amount(AssetType::Moveth(args.amount)),
			)
			.await
			.expect("Failed to lock bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContract::get_bridge_transfer_details_counterparty(
			movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(details.bridge_transfer_id.0, args.bridge_transfer_id.0);
		assert_eq!(details.hash_lock.0, args.hash_lock.0);
		assert_eq!(
			&details.initiator_address.0 .0[32 - args.initiator.len()..],
			&args.initiator,
			"Initiator address does not match"
		);
		assert_eq!(details.recipient_address.0, args.recipient.0.to_vec());
		assert_eq!(details.amount.0, AssetType::Moveth(args.amount));
		assert_eq!(details.state, 1, "Bridge transfer is supposed to be locked but it's not.");

		sleep(Duration::from_secs(5)).await;

		movement_client
			.abort_bridge_transfer(BridgeTransferId(args.bridge_transfer_id.0))
			.await
			.expect("Failed to complete bridge transfer");

		let abort_details = BridgeContract::get_bridge_transfer_details_counterparty(
			movement_client,
			BridgeTransferId(args.bridge_transfer_id.0),
		)
		.await
		.expect("Failed to get bridge transfer details")
		.expect("Expected to find bridge transfer details, but got None");

		assert_eq!(abort_details.bridge_transfer_id.0, args.bridge_transfer_id.0);
		assert_eq!(abort_details.hash_lock.0, args.hash_lock.0);
		assert_eq!(
			&abort_details.initiator_address.0 .0[32 - args.initiator.len()..],
			&args.initiator,
			"Initiator address does not match"
		);
		assert_eq!(abort_details.recipient_address.0, args.recipient.0.to_vec());
		assert_eq!(abort_details.amount.0, AssetType::Moveth(args.amount));

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
		test_utils::fund_and_check_balance(movement_client, 100_000_000_000).await?;
		test_utils::initiate_bridge_transfer_helper(
			movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
			true,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContract::get_bridge_transfer_details_initiator(
			movement_client,
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
		test_utils::fund_and_check_balance(movement_client, 100_000_000_000).await?;
		test_utils::initiate_bridge_transfer_helper(
			movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
			true,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContract::get_bridge_transfer_details_initiator(
			movement_client,
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
			movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
			HashLockPreImage(padded_secret),
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
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
		test_utils::fund_and_check_balance(movement_client, 100_000_000_000).await?;

		let ledger_info = movement_client.rest_client().get_ledger_information().await?;
		println!("Ledger info: {:?}", ledger_info);

		test_utils::initiate_bridge_transfer_helper(
			movement_client,
			args.initiator.0,
			args.recipient.clone(),
			args.hash_lock.0,
			args.amount,
			true,
		)
		.await
		.expect("Failed to initiate bridge transfer");

		let bridge_transfer_id: [u8; 32] =
			test_utils::extract_bridge_transfer_id(movement_client).await?;
		info!("Bridge transfer id: {:?}", bridge_transfer_id);
		let details = BridgeContract::get_bridge_transfer_details_initiator(
			movement_client,
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
			movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
		)
		.await
		.expect("Failed to complete bridge transfer");

		let details = BridgeContract::get_bridge_transfer_details_initiator(
			movement_client,
			BridgeTransferId(MovementHash(bridge_transfer_id).0),
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
