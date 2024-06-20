use anyhow::Context;
use suzuka_full_node::{partial::SuzukaPartialNode, SuzukaFullNode};
use tokio::select;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	#[cfg(feature = "logging")]
	{
		use tracing_subscriber::EnvFilter;

		tracing_subscriber::fmt()
			.with_env_filter(
				EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
			)
			.init();
	}

	// start signal handler to shutdown gracefully and call all destructor
	// on program::exit() the destrutor are not called.
	// End the program to shutdown gracefully when a signal is received.
	let (stop_tx, mut stop_rx) = watch::channel(());
	tokio::spawn({
		let mut sigterm = signal(SignalKind::terminate()).context("Can't register to SIGTERM.")?;
		let mut sigint = signal(SignalKind::interrupt()).context("Can't register to SIGKILL.")?;
		async move {
			loop {
				select! {
					_ = sigterm.recv() => println!("Receive SIGTERM"),
					_ = sigint.recv() => println!("Receive SIGTERM"),
				};
				if let Err(err) = stop_tx.send(()) {
					tracing::warn!("Can't update stop watch channel because :{err}");
					return Err::<(), anyhow::Error>(anyhow::anyhow!(err));
				}
			}
		}
	});

	//Start suzuka node process
	let (gb_jh, run_jh) = {
		let dot_movement = dot_movement::DotMovement::try_from_env()?;
		let path = dot_movement.get_path().join("config.toml");
		let config = suzuka_config::Config::try_from_toml_file(&path).unwrap_or_default();
		tracing::info!("Config loaded:{config:?}");
		let (executor, background_task) = SuzukaPartialNode::try_from_config(config)
			.await
			.context("Failed to create the executor")?;
		let gb_jh = tokio::spawn(background_task);
		let run_jh = tokio::spawn(async move { executor.run().await });
		(gb_jh, run_jh)
	};

	// Wait for a task to end.
	select! {
		_ = stop_rx.changed() => (),
		// manage Suzuka node execution return.
		res = gb_jh => {
			res??;
		},
		res = run_jh => {
			res??;
		},
	}

	Ok(())
}
