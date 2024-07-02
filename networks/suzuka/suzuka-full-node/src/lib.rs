pub mod partial;

#[cfg(test)]
pub mod tests;

pub trait SuzukaFullNode {
	/// Runs the services until crash or shutdown.
	async fn run_services(&self) -> Result<(), anyhow::Error>;

	/// Runs the background tasks until crash or shutdown.
	async fn run_background_tasks(&self) -> Result<(), anyhow::Error>;

	/// Runs the executor until crash or shutdown.
	async fn run_executor(&self) -> Result<(), anyhow::Error>;

	/// Runs the maptos rest api service until crash or shutdown.
	async fn run_movement_rest(&self) -> Result<(), anyhow::Error>;

	/// Runs the full node until crash or shutdown.
	async fn run(&self) -> Result<(), anyhow::Error> {
		// run services and executor concurrently
		tokio::try_join!(self.run_background_tasks(), self.run_services(), self.run_executor(),)?;

		Ok(())
	}
}
