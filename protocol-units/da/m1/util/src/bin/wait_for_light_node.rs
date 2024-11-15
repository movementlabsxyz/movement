use celestia_rpc::HeaderClient;
use godfig::{backend::config_file::ConfigFile, Godfig};
use m1_da_light_node_util::config::M1DaLightNodeConfig;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let mut config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<M1DaLightNodeConfig, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec![]);
	let config = godfig.try_wait_for_ready().await?;
	let client = config.connect_celestia().await?;

	loop {
		let head = client.header_network_head().await?;
		let height: u64 = head.height().into();
		let sync_state = client.header_sync_state().await?;
		info!("Current height: {}, Synced height: {}", height, sync_state.height);
		if height <= sync_state.height {
			break;
		}
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
	}

	Ok(())
}
