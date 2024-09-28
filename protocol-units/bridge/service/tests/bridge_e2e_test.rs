// Add these imports

pub mod ethereum;
pub mod movement;

use crate::ethereum::EthToMovementCallArgs;
use crate::ethereum::SetupEthClient;
use crate::movement::SetupMovementClient;
use alloy::node_bindings::Anvil;
use anyhow::Result;
use aptos_sdk::coin_client::CoinClient;
use aptos_types::account_address::AccountAddress;
use bridge_service::chains::{
	ethereum::{
		client::{Config as EthConfig, EthClient},
		event_monitoring::EthMonitoring,
		types::EthAddress,
	},
	movement::{
		client::{Config as MovementConfig, MovementClient},
		event_monitoring::MovementMonitoring,
		utils::{MovementAddress, MovementHash},
	},
};
use bridge_service::types::Amount;
use bridge_service::types::AssetType;
use bridge_service::types::BridgeAddress;
use bridge_service::types::BridgeTransferId;
use bridge_service::types::HashLock;
use bridge_service::types::HashLockPreImage;
use bridge_service::types::TimeLock;
use tracing_subscriber::EnvFilter;
//use keccak_hash::keccak256;
use alloy::primitives::keccak256;
use tokio::time::{sleep, Duration};
//AlloyProvider, AtomicBridgeInitiator,

#[tokio::test]
async fn test_movement_client_build_and_fund_accounts() -> Result<(), anyhow::Error> {
	let (mut movement_client, mut child) = SetupMovementClient::setup_local_movement_network()
		.await
		.expect("Failed to create SetupMovementClient");
	//
	let rest_client = movement_client.rest_client;
	let coin_client = CoinClient::new(&rest_client);
	let faucet_client = movement_client.faucet_client.write().unwrap();
	let movement_client_signer = movement_client.signer;

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

async fn start_bridge_local(
	eth_config: EthConfig,
) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
	let one_stream = EthMonitoring::build(&eth_config.ws_url.clone().to_string()).await?;

	let one_client = EthClient::new(eth_config).await?;

	let mvt_config = MovementConfig::build_for_test();
	let two_client = MovementClient::new(&mvt_config).await?;

	let two_stream = MovementMonitoring::build(mvt_config).await?;

	let jh = tokio::spawn(async move {
		bridge_service::run_bridge(one_client, one_stream, two_client, two_stream)
			.await
			.unwrap()
	});
	Ok(jh)
}

#[tokio::test]
async fn test_bridge_transfer_eth_movement_happy_path() -> Result<(), anyhow::Error> {
	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	//Configure Movement side
	let (mut movement_client, mut child) = SetupMovementClient::setup_local_movement_network()
		.await
		.expect("Failed to create SetupMovementClient");
	//
	let rest_client = movement_client.rest_client.clone();
	let coin_client = CoinClient::new(&rest_client);
	let movement_client_signer_address = movement_client.signer.address();

	// Deploy smart contract
	let _ = movement_client.publish_for_test();
	{
		let faucet_client = movement_client.faucet_client.write().unwrap();
		faucet_client.fund(movement_client_signer_address, 100_000_000).await?;
	}

	//Configure Ethereum side
	let anvil = Anvil::new().port(8545 as u16).spawn();
	let ws_end_point = anvil.ws_endpoint();
	let mut eth_config = EthConfig::build_for_test();
	let signer: alloy::signers::local::PrivateKeySigner = anvil.keys()[1].clone().into();
	eth_config.signer_private_key = signer;
	let mut eth_client = SetupEthClient::setup_local_ethereum_network(eth_config.clone())
		.await
		.expect("Failed to create SetupEthClient");
	// Deploy smart contract
	let initiator_address = eth_client.deploy_initiator_contract().await;
	let counterpart_address = eth_client.deploy_counterpart_contract().await;
	let weth_address = eth_client.deploy_weth_contract().await;

	eth_client
		.initialize_initiator_contract(
			EthAddress(weth_address),
			EthAddress(eth_client.get_signer_address()),
			1,
		)
		.await
		.unwrap();

	eth_config.initiator_contract = initiator_address.to_string();
	eth_config.counterparty_contract = counterpart_address.to_string();
	eth_config.weth_contract = weth_address.to_string();
	tracing::info!("Eth config:{eth_config:?}");

	// Start bridge.
	let bridge_task_handle = start_bridge_local(eth_config).await.unwrap(); //.expect("Failed to start the bridge");

	// 1) initialize transfer
	// eth_client
	// 	.deposit_weth_and_approve(SetupEthClient::get_initiator_private_key(&anvil), 1)
	// 	.await
	// 	.expect("Failed to deposit WETH");

	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let mov_recipient = MovementAddress(AccountAddress::new(*b"0x00000000000000000000000000face"));

	let amount = Amount(AssetType::EthAndWeth((1, 0)));
	eth_client
		.initiate_bridge_transfer(&anvil, mov_recipient, hash_lock, amount)
		.await
		.expect("Failed to initiate bridge transfer");

	if let Err(e) = child.kill().await {
		tracing::error!("Failed to kill child process: {:?}", e);
	}

	//Wait for the tx to be executed
	let _ = tokio::time::sleep(tokio::time::Duration::from_millis(2000));

	Ok(())
}
