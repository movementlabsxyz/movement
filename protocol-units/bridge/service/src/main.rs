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

	tracing::info!("Start Bridge");
	//define bridge config path
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;
	let mut pathbuff = std::path::PathBuf::from(dot_movement.get_path());
	pathbuff.push(bridge_config::BRIDGE_CONF_FOLDER);
	dot_movement.set_path(pathbuff);

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);
	let bridge_config: Config = godfig.try_wait_for_ready().await?;
	tracing::info!("Bridge config loaded: {bridge_config:?}");

	let one_stream = EthMonitoring::build(&bridge_config.eth).await.unwrap();

	let one_client = EthClient::new(&bridge_config.eth).await.unwrap();

	let two_client = MovementClient::new(&bridge_config.movement).await.unwrap();

	let two_stream = MovementMonitoring::build(&bridge_config.movement).await.unwrap();

	tracing::info!("Bridge Eth and Movement Inited. Starting bridge loop.");
	bridge_service::run_bridge(one_client, one_stream, two_client, two_stream).await?;
	Ok(())
}
