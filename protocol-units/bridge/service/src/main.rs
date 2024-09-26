#![allow(unused_imports)]
use anyhow::Result;
use ethereum_bridge::client::{Config as EthConfig, EthClient};
use ethereum_bridge::event_monitoring::EthInitiatorMonitoring;
use movement_bridge::client::{Config as MovementConfig, MovementClient};

#[tokio::main]
async fn main() -> Result<()> {
	// let eth_ws_url = "";
	// let one_stream = EthMonitoring::build(eth_ws_url).await?;
	//
	// let eth_config = EthConfig::build_for_test();
	// let one_client = EthClient::new(eth_config).await?;
	//
	// let mvt_config = MovementConfig::build_for_test();
	// let two_client = MovementClient::new(&mvt_config).await?;
	//
	// let two_stream = MovementMonitoring::build(mvt_config).await?;
	//
	// //bridge_service::run_bridge(one_client, one_stream, two_client, two_stream).await?;
	Ok(())
}
