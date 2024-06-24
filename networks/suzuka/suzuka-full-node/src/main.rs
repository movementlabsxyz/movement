use anyhow::Context;
use std::process::ExitCode;
use suzuka_full_node::{partial::SuzukaPartialNode, SuzukaFullNode};
use tokio::select;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::watch;

fn main() -> Result<ExitCode, anyhow::Error> {
	let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

	#[cfg(feature = "logging")]
	{
		use tracing_subscriber::EnvFilter;

		tracing_subscriber::fmt()
			.with_env_filter(
				EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
			)
			.init();
	}

	if let Err(err) = runtime.block_on(start_suzuka()) {
		tracing::error!("Suzuka node main task exit with an error : {err}",);
	}

	// Terminate all running task.
	runtime.shutdown_background();
	Ok(ExitCode::SUCCESS)
}

async fn start_suzuka() -> Result<(), anyhow::Error> {
	// start signal handler to shutdown gracefully and call all destructor
	// on program::exit() the destrutor are not called.
	// End the program to shutdown gracefully when a signal is received.
	let (stop_tx, mut stop_rx) = watch::channel(());
	tokio::spawn({
		let mut sigterm = signal(SignalKind::terminate()).context("Can't register to SIGTERM.")?;
		let mut sigint = signal(SignalKind::interrupt()).context("Can't register to SIGKILL.")?;
		let mut sigquit = signal(SignalKind::quit()).context("Can't register to SIGKILL.")?;
		async move {
			loop {
				select! {
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

	//Start suzuka node process
	let dot_movement = dot_movement::DotMovement::try_from_env()?;
	let config = dot_movement.try_get_config_from_json::<suzuka_config::Config>()?;
	let (executor, background_task) = SuzukaPartialNode::try_from_config(config)
		.await
		.context("Failed to create the executor")?;
	let gb_jh = tokio::spawn(background_task);
	let run_jh = tokio::spawn(async move { executor.run().await });

	// Wait for a task to end.
	select! {
		_ = stop_rx.changed() =>(),
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
