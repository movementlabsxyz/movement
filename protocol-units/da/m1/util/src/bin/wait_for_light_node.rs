use celestia_rpc::HeaderClient;
use m1_da_light_node_util::Config;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let config = Config::try_from_env_toml_file()?;
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
