use anyhow::Context;
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_types::application;
use std::future::Future;
use std::pin::Pin;
use suzuka_config::Config;
use suzuka_full_node_setup::{local::Local, SuzukaFullNodeSetupOperations};
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::watch;
use tracing::info;

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
				tracing::info!("Received terminate Signal");
				if let Err(err) = stop_tx.send(()) {
					tracing::warn!("Can't update stop watch channel because :{err}");
					return Err::<(), anyhow::Error>(anyhow::anyhow!(err));
				}
			}
		}
	});

	// get the config file
	let dot_movement = dot_movement::DotMovement::try_from_env()?;

	let config_file = dot_movement.try_get_or_create_config_file().await?;

	// get a matching godfig object
	let godfig: Godfig<Config, ConfigFile> = Godfig::new(ConfigFile::new(config_file), vec![]);

	// Apply all of the setup steps
	let (anvil_join_handle, sync_task) = godfig
		.try_transaction_with_result(|config| async move {
			tracing::info!("Config: {:?}", config);
			let config = config.unwrap_or_default();
			tracing::info!("Config: {:?}", config);

			// set up sync
			let sync_task: Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> =
				if let Some(sync_str) = config.syncing.movement_sync.clone() {
					let mut leader_follower_split = sync_str.split("::");
					let is_leader = leader_follower_split.next().context(
						"MOVEMENT_SYNC environment variable must be in the format <leader|follower>::<sync-pattern>",
					)? == "leader";

					let mut bucket_arrow_glob = leader_follower_split.next().context(
						"MOVEMENT_SYNC environment variable must be in the format <leader|follower>::<sync-pattern>",
					)?.split("<=>");

					let bucket = bucket_arrow_glob.next().context(
						"MOVEMENT_SYNC environment variable must be in the format <bucket>,<glob>",
					)?;
					let glob = bucket_arrow_glob.next().context(
						"MOVEMENT_SYNC environment variable must be in the format <bucket>,<glob>",
					)?;

					info!("Syncing with bucket: {}, glob: {}", bucket, glob);
					let sync_task = dot_movement
						.sync(is_leader, glob, bucket.to_string(), application::Id::suzuka())
						.await?;
					Box::pin(async {
						sync_task.await?;
						Ok(())
					})
				} else {
					Box::pin(async {
						info!("No sync task configured, skipping.");
						futures::future::pending::<Result<(), anyhow::Error>>().await
					})
				};

			// set up anvil
			let (config, anvil_join_handle) = Local::default().setup(dot_movement, config).await?;

			Ok((Some(config), (anvil_join_handle, sync_task)))
		})
		.await?;

	info!("Initial setup complete, orchestrating services.");

	// Use tokio::select! to wait for either the handle or a cancellation signal
	tokio::select! {
		res = anvil_join_handle => {
			tracing::info!("Anvil task finished.");
			res??;
		}
		_ = stop_rx.changed() => {
			tracing::info!("Cancellation received, killing anvil task.");
		}
		// sync task
		_ = sync_task => {
			tracing::info!("Sync task finished.");
		}
	}

	Ok(())
}
