use super::partial::MovementPartialNode;
use anyhow::Context;
use godfig::{backend::config_file::ConfigFile, Godfig};
use movement_config::Config;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;

#[derive(Clone)]
pub struct Manager {
	godfig: Godfig<Config, ConfigFile>,
}

// Implements a very simple manager using a marker strategy pattern.
impl Manager {
	pub async fn new(file: tokio::fs::File) -> Result<Self, anyhow::Error> {
		let godfig = Godfig::new(ConfigFile::new(file), vec![]);
		Ok(Self { godfig })
	}

	pub async fn try_run(&self) -> Result<(), anyhow::Error> {
		let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(());
		tokio::spawn({
			let mut sigterm =
				signal(SignalKind::terminate()).context("can't register to SIGTERM.")?;
			let mut sigint =
				signal(SignalKind::interrupt()).context("can't register to SIGKILL.")?;
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

		let config = self.godfig.try_wait_for_ready().await?;

		let node = MovementPartialNode::try_from_config(config)
			.await
			.context("Failed to create the executor")?;

		let join_handle = tokio::spawn(node.run());

		// Use tokio::select! to wait for either the handle or a cancellation signal
		tokio::select! {
			_ = stop_rx.changed() =>(),
			// manage Movement node execution return.
			res = join_handle => {
				res??;
			},
		};

		Ok(())
	}
}
