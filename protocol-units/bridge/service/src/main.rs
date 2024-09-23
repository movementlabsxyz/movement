use anyhow::Result;
//use bridge_serivice::{setup_bridge_service, SetupBridgeService};
//use bridge_shared::bridge_service::BridgeServiceConfig;

mod swapstate;

#[tokio::main]
async fn main() -> Result<()> {
	// let config = BridgeServiceConfig::default();
	// let SetupBridgeService(
	// 	mut _bridge_service,
	// 	mut _eth_client,
	// 	mut _movement_client,
	// 	ethereum_chain,
	// 	movement_chain,
	// ) = setup_bridge_service(config).await;

	// tokio::spawn(ethereum_chain);
	// tokio::spawn(movement_chain);

	swapstate::run_bridge("eth_ws_url").await?;
	Ok(())
}
