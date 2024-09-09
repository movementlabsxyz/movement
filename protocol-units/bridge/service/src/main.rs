use bridge_serivice::{setup_bridge_service, SetupBridgeServiceResult};
use bridge_shared::bridge_service::BridgeServiceConfig;

#[tokio::main]
async fn main() -> Result<()> {
	let config = BridgeServiceConfig::default();
	let SetupBridgeServiceResult(
		mut bridge_service,
		mut eth_client,
		mut movement_client,
		ethereuem_chain,
		movement_chain,
	) = setup_bridge_service(config);

	tokio::spawn(ethereum_chain);
	tokio::spawn(movement_chain);
	Ok(())
}
