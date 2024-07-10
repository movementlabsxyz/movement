use crate::SuzukaFullNode;
use anyhow::Context;
use super::partial::SuzukaPartialNode;
use godfig::{
    Godfig,
    backend::config_file::ConfigFile
};
use suzuka_config::Config;
use maptos_dof_execution::v1::Executor;

#[derive(Clone)]
pub struct Manager<Dof>
    where 
    Dof : SuzukaFullNode {
    godfig: Godfig<Config, ConfigFile>,
    _marker : std::marker::PhantomData<Dof>,
}

// Implements a very simple manager using a marker strategy pattern.
impl Manager<SuzukaPartialNode<Executor>> {
    pub async fn new(file : tokio::fs::File) -> Result<Self, anyhow::Error> {
        let godfig = Godfig::new(ConfigFile::new(file), vec![]);
        Ok(Self {
            godfig,
            _marker: std::marker::PhantomData,
        })
    }

    pub async fn try_run(&self) -> Result<(), anyhow::Error> {
        
        let config = self.godfig.try_wait_for_ready().await?;
        
        let (executor, background_task) = SuzukaPartialNode::try_from_config(config)
		.await
		.context("Failed to create the executor")?;

	    tokio::spawn(background_task);

	    executor.run().await.context("Failed to run suzuka")?;

        Ok(())
    }
}