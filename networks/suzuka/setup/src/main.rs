use suzuka_full_node_setup::{local::Local, SuzukaFullNodeSetupOperations};
use godfig::{
	Godfig,
	backend::config_file::ConfigFile
};
use suzuka_config::Config;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::watch;
use anyhow::Context;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	use tracing_subscriber::EnvFilter;

	tracing_subscriber::fmt()
		.with_env_filter(
			EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
		)
		.init();

	let (stop_tx, mut stop_rx) = watch::channel(());
	tokio::spawn({
		let mut sigterm = signal(SignalKind::terminate()).context("Can't register to SIGTERM.")?;
		let mut sigint = signal(SignalKind::interrupt()).context("Can't register to SIGKILL.")?;
		async move {
			loop {
				tokio::select! {
					_ = sigterm.recv() => (),
					_ = sigint.recv() => (),
				};
				tracing::info!("Receive Terminate Signal");
				if let Err(err) = stop_tx.send(()) {
					tracing::warn!("Can't update stop watch channel because :{err}");
					return Err::<(), anyhow::Error>(anyhow::anyhow!(err));
				}
			}
		}
	});

	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let mut config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig : Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

	// Apply all of the setup steps
	let anvil_join_handle = godfig.try_transaction_with_result(|config| async move {

			tracing::info!("Config: {:?}", config);
			let config = config.unwrap_or_default();
			tracing::info!("Config: {:?}", config);

			let (config, anvil_join_handle) = Local::default().setup(dot_movement, config).await?;
		
			Ok((Some(config), anvil_join_handle))

		}).await?;

	// Use tokio::select! to wait for either the handle or a cancellation signal
	tokio::select! {
		res = anvil_join_handle => {
			tracing::info!("Anvil task finished.");
			res??;
		}
		_ = stop_rx.changed() => {
			tracing::info!("Cancellation received, killing anvil task.");
		}
	}

	Ok(())
}
