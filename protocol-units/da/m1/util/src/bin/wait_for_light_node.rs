use celestia_rpc::HeaderClient;
use m1_da_light_node_util::Config;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

	use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let path = dot_movement.get_path().join("config.toml");
	let config = Config::try_from_toml_file(&path).unwrap_or_default();
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
