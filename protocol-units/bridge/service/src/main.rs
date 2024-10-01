use anyhow::Result;
use bridge_service::chains::ethereum::client::{Config as EthConfig, EthClient};
use bridge_service::chains::ethereum::event_monitoring::EthMonitoring;
use bridge_service::chains::movement::client::{Config as MovementConfig, MovementClient};
use bridge_service::chains::movement::event_monitoring::MovementMonitoring;

#[tokio::main]
async fn main() -> Result<()> {
	let eth_ws_url = "";
	let one_stream = EthMonitoring::build(&bridge_config.eth).await?;

	let bridge_config = bridge_config::Config::default();
	let one_client = EthClient::new(&bridge_config.eth).await?;

	let two_client = MovementClient::new(&bridge_config.mvt).await?;

	let two_stream = MovementMonitoring::build(&bridge_config.mvt).await?;

	bridge_service::run_bridge(one_client, one_stream, two_client, two_stream).await?;
	Ok(())
}
