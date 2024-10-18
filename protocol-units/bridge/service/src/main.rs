use anyhow::Result;
use bridge_config::{common::movement::MovementConfig, Config};
use bridge_service::chains::ethereum::client::EthClient;
use bridge_service::chains::ethereum::event_monitoring::EthMonitoring;
use bridge_service::chains::movement::client_framework::MovementClientFramework;
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

	tracing::info!("Start Bridge");
	//define bridge config path
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;
	let pathbuff = bridge_config::get_config_path(&dot_movement);
	dot_movement.set_path(pathbuff);

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);
	let mut bridge_config: Config = godfig.try_wait_for_ready().await?;

	//bridge_config.movement = MovementConfig::suzuka();

	tracing::info!("Bridge config loaded: {bridge_config:?}");

	let one_stream = EthMonitoring::build(&bridge_config.eth).await.unwrap();

	let one_client = EthClient::new(&bridge_config.eth).await.unwrap();

	let two_client = MovementClientFramework::new(&bridge_config.movement).await.unwrap();

	let two_stream = MovementMonitoring::build(&bridge_config.movement).await.unwrap();

	tracing::info!("Bridge Eth and Movement Inited. Starting bridge loop.");
	bridge_service::run_bridge(one_client, one_stream, two_client, two_stream).await?;
	Ok(())
}
