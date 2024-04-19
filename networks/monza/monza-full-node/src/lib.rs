pub mod partial;

pub trait MonzaFullNode {

    /// Runs the services until crash or shutdown.
    async fn run_services(&self) -> Result<(), anyhow::Error>;

    /// Runs the executor until crash or shutdown.
    async fn run_executor(&self) -> Result<(), anyhow::Error>;

    /// Runs the full node until crash or shutdown.
    async fn run(&self) -> Result<(), anyhow::Error> {
        
        // run services and executor concurrently
        tokio::try_join!(
            self.run_services(),
            self.run_executor()
        )?;

        Ok(())
    }

}