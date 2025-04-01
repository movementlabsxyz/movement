use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_node::server::run_server;
use movement_da_sequencer_node::whitelist::Whitelist;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	tracing::info!("Start Bridge");

	// Define da-sequencer config path
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;
	let pathbuff = movement_da_sequencer_config::get_config_path(&dot_movement);
	dot_movement.set_path(pathbuff);

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<DaSequencerConfig, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec![]);
	let da_sequencer_config: DaSequencerConfig = godfig.try_wait_for_ready().await?;

	// Initialize whitelist
	let mut whitelist_path = dot_movement.get_path().to_path_buf();
	whitelist_path.push("default_signer_address_whitelist");
	let whitelist = Whitelist::from_file_and_spawn_reload_thread(whitelist_path)?;

	let (request_tx, request_rx) = mpsc::channel(100);
	// Start gprc server
	let grpc_address = da_sequencer_config.movement_da_sequencer_listen_address;
	let grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist).await });

	//Start the main loop
	todo!();

	Ok(())
}
