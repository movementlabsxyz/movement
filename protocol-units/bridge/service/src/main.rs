use anyhow::Result;
use bridge_service::{setup_bridge_service, SetupBridgeService};
use bridge_shared::bridge_service::BridgeServiceConfig;

#[tokio::main]
async fn main() -> Result<()> {
	let config = BridgeServiceConfig::default();
	let SetupBridgeService(
		mut _bridge_service,
		mut _eth_client,
		mut _movement_client,
		ethereum_chain,
		movement_chain,
	) = setup_bridge_service(config).await;

	tokio::spawn(ethereum_chain);
	tokio::spawn(movement_chain);
	Ok(())
}
