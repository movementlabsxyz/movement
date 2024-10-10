use alloy::primitives::keccak256;
use alloy::primitives::{FixedBytes, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy_network::EthereumWallet;
use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_types::account_address::AccountAddress;
use bridge_config::Config;
use bridge_integration_tests::HarnessEthClient;
use bridge_integration_tests::TestHarness;
use bridge_service::chains::bridge_contracts::BridgeContractError;
use bridge_service::chains::ethereum::types::AtomicBridgeInitiator;
use bridge_service::chains::ethereum::utils::send_transaction;
use bridge_service::chains::ethereum::utils::send_transaction_rules;
use bridge_service::chains::{
	ethereum::{client::EthClient, event_monitoring::EthMonitoring, types::EthAddress},
	movement::{
		client::MovementClient, event_monitoring::MovementMonitoring, utils::MovementAddress,
	},
};
use bridge_service::types::Amount;
use bridge_service::types::AssetType;
use bridge_service::types::BridgeAddress;
use bridge_service::types::HashLock;
use bridge_service::types::HashLockPreImage;
use tokio_stream::StreamExt;
use tracing_subscriber::EnvFilter;

async fn start_bridge_local(config: &Config) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
	let one_stream = EthMonitoring::build(&config.eth).await?;
	let one_client = EthClient::new(&config.eth).await?;
	let two_client = MovementClient::new(&config.movement).await?;

	let two_stream = MovementMonitoring::build(&config.movement).await?;

	let jh = tokio::spawn(async move {
		bridge_service::run_bridge(one_client, one_stream, two_client, two_stream)
			.await
			.unwrap()
	});
	Ok(jh)
}

async fn initiate_eth_bridge_transfer(
	config: &Config,
	initiator_privatekey: PrivateKeySigner,
	recipient: MovementAddress,
	hash_lock: HashLock,
	amount: Amount,
) -> Result<(), anyhow::Error> {
	let initiator_address = initiator_privatekey.address();
	let rpc_provider = ProviderBuilder::new()
		.with_recommended_fillers()
		.wallet(EthereumWallet::from(initiator_privatekey))
		.on_builtin(&config.eth.eth_rpc_connection_url())
		.await?;

	let contract =
		AtomicBridgeInitiator::new(config.eth.eth_initiator_contract.parse()?, &rpc_provider);

	let initiator_address = BridgeAddress(EthAddress(initiator_address));

	let recipient_address = BridgeAddress(Into::<Vec<u8>>::into(recipient));

	let recipient_bytes: [u8; 32] =
		recipient_address.0.try_into().expect("Recipient address must be 32 bytes");
	let call = contract
		.initiateBridgeTransfer(
			U256::from(amount.weth_value()),
			FixedBytes(recipient_bytes),
			FixedBytes(hash_lock.0),
		)
		.value(U256::from(amount.eth_value()))
		.from(*initiator_address.0);
	let _ = send_transaction(
		call,
		&send_transaction_rules(),
		config.eth.transaction_send_retries,
		config.eth.gas_limit as u128,
	)
	.await
	.map_err(|e| BridgeContractError::GenericError(format!("Failed to send transaction: {}", e)))?;
	Ok(())
}

#[tokio::test]
async fn test_bridge_transfer_eth_movement_happy_path() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let (eth_client_harness, mvt_client_harness, config) =
		TestHarness::new_with_eth_and_movement().await?;

	let movement_client_signer_address = mvt_client_harness.movement_client.signer().address();

	{
		let faucet_client = mvt_client_harness.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000).await?;
	}

	// 1) initialize transfer
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let mov_recipient = MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face"));

	let amount = Amount(AssetType::EthAndWeth((1, 0)));
	initiate_eth_bridge_transfer(
		&config,
		HarnessEthClient::get_initiator_private_key(&config),
		mov_recipient,
		hash_lock,
		amount,
	)
	.await
	.expect("Failed to initiate bridge transfer");

	//Wait for the tx to be executed
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

	//send counter complete event.

	Ok(())
}

use aptos_sdk::crypto::ed25519::Ed25519PublicKey;
#[tokio::test]
async fn test_movement_event() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	println!("Start test_movement_event",);

	let config = TestHarness::read_bridge_config().await?;
	println!("after test_movement_event",);

	//	1) initialize transfer
	// let hash_lock_pre_image = HashLockPreImage::random();
	// let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	// let mov_recipient = MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face"));

	// let amount = Amount(AssetType::EthAndWeth((1, 0)));
	// initiate_eth_bridge_transfer(
	// 	&config,
	// 	HarnessEthClient::get_initiator_private_key(&config),
	// 	mov_recipient,
	// 	hash_lock,
	// 	amount,
	// )
	// .await
	// .expect("Failed to initiate bridge transfer");

	use bridge_integration_tests::MovementToEthCallArgs;

	let mut movement_client = MovementClient::new(&config.movement).await.unwrap();

	let args = MovementToEthCallArgs::default();
	// let signer_privkey = config.movement.movement_signer_key.clone();
	// let sender_address = format!("0x{}", Ed25519PublicKey::from(&signer_privkey).to_string());
	// let sender_address = movement_client.signer().address();
	//		test_utils::fund_and_check_balance(&mut mvt_client_harness, 100_000_000_000).await?;
	bridge_integration_tests::utils::initiate_bridge_transfer_helper(
		&mut movement_client,
		args.initiator.0,
		args.recipient.clone(),
		args.hash_lock.0,
		args.amount,
		true,
	)
	.await
	.expect("Failed to initiate bridge transfer");

	//Wait for the tx to be executed
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
	let event_type = format!(
		"{}::atomic_bridge_initiator::BridgeTransferStore",
		config.movement.movement_native_address
	);

	// let signer_privkey = config.movement.movement_signer_key.clone();
	// let signer_public_key = format!("0x{}", Ed25519PublicKey::from(&signer_privkey).to_string());

	// println!("signer_public_key {signer_public_key}",);

	// let res = fetch_account_events(
	// 	&config.movement.mvt_rpc_connection_url(),
	// 	&signer_public_key,
	// 	&event_type,
	// )
	// .await;
	// println!("res: {res:?}",);

	// let res = test_get_events_by_account_event_handle(
	// 	&config.movement.mvt_rpc_connection_url(),
	// 	"0xf90391c81027f03cdea491ed8b36ffaced26b6df208a9b569e5baf2590eb9b16",
	// 	&event_type,
	// )
	// .await;
	// println!("res: {res:?}",);

	let res = test_get_events_by_account_event_handle(
		&config.movement.mvt_rpc_connection_url(),
		&config.movement.movement_native_address,
		&event_type,
	)
	.await;
	println!("res: {res:?}",);

	let res = fetch_account_events(
		&config.movement.mvt_rpc_connection_url(),
		&config.movement.movement_native_address,
		&event_type,
	)
	.await;
	println!("res: {res:?}",);

	// let mut one_stream = MovementMonitoring::build(&config.movement).await?;

	// //listen to event.
	// let mut error_counter = 0;
	// loop {
	// 	tokio::select! {
	// 		// Wait on chain one events.
	// 		Some(one_event_res) = one_stream.next() =>{
	// 			match one_event_res {
	// 				Ok(one_event) => {
	// 					println!("Receive event {:?}", one_event);
	// 				}
	// 				Err(err) => {
	// 					println!("Receive error {:?}", err);
	// 					error_counter +=1;
	// 					if error_counter > 5 {
	// 						break;
	// 					}
	// 				}
	// 			}
	// 		}
	// 	}
	// }

	Ok(())
}

async fn test_get_events_by_account_event_handle(
	rest_url: &str,
	account_address: &str,
	event_type: &str,
) {
	// let url =
	// 	format!("{}/v1/accounts/{}/events?event_type={}", rest_url, account_address, event_type);
	let url = format!(
		"{}/v1/accounts/{}/events/{}/bridge_transfer_initiated_events",
		rest_url, account_address, event_type
	);

	println!("url: {:?}", url);
	let client = reqwest::Client::new();

	// Send the GET request
	let response = client
		.get(&url)
		.query(&[("start", "0"), ("limit", "10")])
		.send()
		.await
		.unwrap()
		.text()
		.await;
	println!("Account direct response: {response:?}",);
}

use aptos_sdk::rest_client::Client;
use std::str::FromStr;

async fn fetch_account_events(rest_url: &str, account_address: &str, event_type: &str) {
	// Initialize the RestClient
	let node_connection_url = url::Url::from_str(rest_url).unwrap();
	let client = Client::new(node_connection_url); // Use the correct node URL
	let native_address = AccountAddress::from_hex_literal(account_address).unwrap();

	// Get the events for the specified account
	let response = client
		.get_account_events(
			native_address,
			event_type,
			"bridge_transfer_initiated_events",
			Some(1),
			None,
		)
		.await;

	println!("response{response:?}",);

	// let core_code_address: AccountAddress = AccountAddress::from_hex_literal("0x1").unwrap();
	// let response = client
	// 	.get_account_events_bcs(
	// 		core_code_address,
	// 		"0x1::block::BlockResource",
	// 		"new_block_events",
	// 		Some(1),
	// 		None,
	// 	)
	// 	.await;
	// println!("new_block_events response: {response:?}",);
}
