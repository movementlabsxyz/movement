use alloy::{
	node_bindings::Anvil,
	primitives::{address, keccak256},
};
use anyhow::Result;
use bridge_service::chains::{
	bridge_contracts::BridgeContract,
	ethereum::types::{EthAddress, EthHash},
};
use bridge_service::chains::{
	ethereum::{
		client::{Config as EthConfig, EthClient},
		event_monitoring::{Config as EthMonitoringConfig, EthMonitoring},
	},
	movement::{
		client::{Config as MovementConfig, MovementClient},
		event_monitoring::MovementMonitoring,
	},
};
use tokio_stream::StreamExt;

use bridge_service::types::{Amount, AssetType, BridgeAddress, HashLock};
use harness::TestHarness;
mod harness;
mod utils;

#[tokio::test]
async fn test_should_receive_event() -> Result<()> {
	let mut harness: TestHarness = TestHarness::new_only_eth().await;
	let anvil = Anvil::new().port(harness.rpc_port()).spawn();

	let signer_address = harness.set_eth_signer(anvil.keys()[0].clone());
	harness.deploy_init_contracts().await;

	let eth_stream: EthMonitoring = EthMonitoring::build(EthMonitoringConfig::default()).await?;

	let eth_config = EthConfig::build_for_test();
	let eth_client = EthClient::new(eth_config).await?;

	let mvt_config = MovementConfig::build_for_test();
	let mvt_client = MovementClient::new(&mvt_config).await?;

	let mvt_stream = MovementMonitoring::build(mvt_config).await?;

	//Start the relayer
	bridge_service::run_bridge(eth_client, eth_stream, mvt_client, mvt_stream).await?;

	let recipient = address!("70997970c51812dc3a010c7d01b50e0d17dc79c8");
	let recipient_bytes: Vec<u8> = recipient.to_string().as_bytes().to_vec();
	let secret = "secret".to_string();
	let hash_lock = keccak256(secret.as_bytes());
	let hash_lock: [u8; 32] = hash_lock.into();

	harness
		.eth_client_mut()
		.expect("Failed to get EthClient")
		.initiate_bridge_transfer(
			BridgeAddress(EthAddress(signer_address)),
			BridgeAddress(recipient_bytes),
			HashLock(EthHash(hash_lock).0),
			Amount(AssetType::EthAndWeth((42, 0))),
		)
		.await
		.expect("Failed to initiate bridge transfer");

	Ok(())
}
