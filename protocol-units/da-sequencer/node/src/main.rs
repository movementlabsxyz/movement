use anyhow::Context;
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_da_sequencer_config::DaSequencerConfig;
use movement_da_sequencer_node::celestia::mock::CelestiaMock;
use movement_da_sequencer_node::run;
use movement_da_sequencer_node::server::run_server;
use movement_da_sequencer_node::storage::Storage;
use movement_da_sequencer_node::whitelist::Whitelist;
use std::error::Error;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::mpsc;

pub const GRPC_REQUEST_CHANNEL_SIZE: usize = 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	// Define da-sequencer config path
	let mut dot_movement = dot_movement::DotMovement::try_from_env()?;
	let pathbuff = movement_da_sequencer_config::get_config_path(&dot_movement);
	tracing::info!("Start Da Sequencer with config file in {pathbuff:?}.");
	dot_movement.set_path(pathbuff);

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// Get a matching godfig object
	let godfig: Godfig<DaSequencerConfig, ConfigFile> =
		Godfig::new(ConfigFile::new(config_file), vec![]);
	let da_sequencer_config: DaSequencerConfig = godfig.try_wait_for_ready().await?;

	let dotmovement_path = dot_movement.get_path().to_path_buf();

	// Initialize whitelist
	let whitelist_path = dotmovement_path.join(&da_sequencer_config.whitelist_relative_path);
	let whitelist = Whitelist::from_file_and_spawn_reload_thread(whitelist_path)?;

	let (request_tx, request_rx) = mpsc::channel(GRPC_REQUEST_CHANNEL_SIZE);
	// Start gprc server
	let grpc_address = da_sequencer_config.grpc_listen_address;
	let grpc_jh =
		tokio::spawn(async move { run_server(grpc_address, request_tx, whitelist).await });

	//Start the main loop
	let db_storage_path = dotmovement_path.join(&da_sequencer_config.db_storage_relative_path);

	let storage = Storage::try_new(&db_storage_path)?;

	// TODO Use Celestia Mock for now
	let celestia_mock = CelestiaMock::new();
	let loop_jh = tokio::spawn(run(da_sequencer_config, request_rx, storage, celestia_mock));

	let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(());
	tokio::spawn({
		let mut sigterm = signal(SignalKind::terminate()).context("can't register to SIGTERM.")?;
		let mut sigint = signal(SignalKind::interrupt()).context("can't register to SIGKILL.")?;
		let mut sigquit = signal(SignalKind::quit()).context("can't register to SIGKILL.")?;
		async move {
			loop {
				tokio::select! {
					_ = sigterm.recv() => (),
					_ = sigint.recv() => (),
					_ = sigquit.recv() => (),
				};
				tracing::info!("Receive Terminate Signal");
				if let Err(err) = stop_tx.send(()) {
					tracing::warn!("Can't update stop watch channel because :{err}");
					return Err::<(), anyhow::Error>(anyhow::anyhow!(err));
				}
			}
		}
	});
	// Use tokio::select! to wait for either the handle or a cancellation signal
	tokio::select! {
		_ = stop_rx.changed() =>(),
		res = grpc_jh => {
			tracing::error!("Grpc server exit because :{res:?}");
		}
		res = loop_jh => {
			tracing::error!("Da Sequencer main process exit because :{res:?}");
		}
	};

	Ok(())
}
