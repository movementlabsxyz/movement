use anyhow::Result;
use bridge_config::Config;
use bridge_indexer_db::run_indexer_client;
use bridge_service::chains::ethereum::event_monitoring::EthMonitoring;
use bridge_service::chains::movement::event_monitoring::MovementMonitoring;
use bridge_service::rest::BridgeRest;
use bridge_util::chains::check_monitoring_health;
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

	// Define bridge config path
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;
	let pathbuff = bridge_config::get_config_path(&dot_movement);
	dot_movement.set_path(pathbuff);

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);
	let bridge_config: Config = godfig.try_wait_for_ready().await?;

	tracing::info!("Bridge config loaded: {bridge_config:?}");

	let (eth_client_health_tx, eth_client_health_rx) = tokio::sync::mpsc::channel(10);
	let (mvt_client_health_tx, mvt_client_health_rx) = tokio::sync::mpsc::channel(10);
	let eth_stream = EthMonitoring::build(&bridge_config.eth, eth_client_health_rx).await.unwrap();
	let mvt_stream = MovementMonitoring::build(&bridge_config.movement, mvt_client_health_rx)
		.await
		.unwrap();

	// Initialize the Rpc health check service
	let (eth_rest_health_tx, eth_rest_health_rx) = tokio::sync::mpsc::channel(10);
	let (mvt_rest_health_tx, mvt_rest_health_rx) = tokio::sync::mpsc::channel(10);
	// Create and run the REST service
	let url = format!(
		"{}:{}",
		bridge_config.indexer.rest_listener_hostname, bridge_config.indexer.rest_port
	);
	let rest_service = BridgeRest::new(url, eth_rest_health_tx, mvt_rest_health_tx)?;
	let rest_service_future = rest_service.run_service();
	let rest_jh = tokio::spawn(rest_service_future);

	tracing::info!("Bridge Eth and Movement Inited. Starting bridge loop.");

	// Start Monitoring health check.
	let eth_healh_check_jh =
		tokio::spawn(check_monitoring_health("Eth", eth_client_health_tx, eth_rest_health_rx));
	let mvt_healh_check_jh =
		tokio::spawn(check_monitoring_health("Mvt", mvt_client_health_tx, mvt_rest_health_rx));

	tracing::info!("Bridge Eth and Movement Inited. Starting bridge loop.");

	// Start indexer
	let indexer_jh = tokio::spawn(run_indexer_client(bridge_config, eth_stream, mvt_stream, None));

	tokio::select! {
		res = eth_healh_check_jh => {
			tracing::error!("Heath check Eth monitoring exit because :{res:?}");
		}
		res = mvt_healh_check_jh => {
			tracing::error!("Heath check Eth monitoring exit because :{res:?}");
		}
		res = rest_jh => {
			tracing::error!("Heath check Rest server exit because :{res:?}");
		}
		res = indexer_jh => {
			tracing::error!("Indexer loop exit because :{res:?}");
		}
	};

	Ok(())
}
