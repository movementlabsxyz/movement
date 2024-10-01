use anyhow::Result;
use bridge_config::Config;
use bridge_service::chains::ethereum::client::EthClient;
use bridge_service::chains::ethereum::event_monitoring::EthMonitoring;
use bridge_service::chains::movement::client::MovementClient;
use bridge_service::chains::movement::event_monitoring::MovementMonitoring;
use godfig::{backend::config_file::ConfigFile, Godfig};

#[tokio::main]
async fn main() -> Result<()> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec!["mcr_settlement".to_string()]);
	let bridge_config: Config = godfig.try_wait_for_ready().await?;

	let one_stream = EthMonitoring::build(&bridge_config.eth).await?;

	let one_client = EthClient::new(&bridge_config.eth).await?;

	let two_client = MovementClient::new(&bridge_config.movement).await?;

	let two_stream = MovementMonitoring::build(&bridge_config.movement).await?;

	bridge_service::run_bridge(one_client, one_stream, two_client, two_stream).await?;
	Ok(())
}
