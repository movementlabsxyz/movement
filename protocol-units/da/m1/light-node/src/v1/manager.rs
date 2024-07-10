use super::{LightNodeV1, LightNodeV1Operations};
use godfig::{
    Godfig,
    backend::config_file::ConfigFile
};
use m1_da_light_node_util::config::Config;

#[derive(Clone)]
pub struct Manager<LightNode> 
where LightNode: LightNodeV1Operations {
    godfig: Godfig<Config, ConfigFile>,
    _marker : std::marker::PhantomData<LightNode>,
}

// Implements a very simple manager using a marker strategy pattern.
impl Manager<LightNodeV1> {
    pub async fn new(file : tokio::fs::File) -> Result<Self, anyhow::Error> {
        let godfig = Godfig::new(ConfigFile::new(file), vec![
            "m1_da_light_node_config".to_string() // in this example this comes from the structuring of the config file
        ]);
        Ok(Self {
            godfig,
            _marker: std::marker::PhantomData,
        })
    }

    pub async fn try_light_node(&self) -> Result<LightNodeV1, anyhow::Error> {
        let config = self.godfig.try_wait_for_ready().await?;
        LightNodeV1::try_from_config(config).await
    }

    pub async fn try_run(&self) -> Result<(), anyhow::Error> {
        let light_node = self.try_light_node().await?;
        light_node.run().await
    }
}