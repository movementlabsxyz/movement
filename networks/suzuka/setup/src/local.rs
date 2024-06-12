use commander::run_command;
use dot_movement::DotMovement;
use tokio::fs;
use tracing::info;
use celestia_types::nmt::Namespace;
use crate::SuzukaFullNodeSetupOperations;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct Local;

impl Local {
    
    pub fn new() -> Self {
        Self
    }

}

impl SuzukaFullNodeSetupOperations for Local {
    async fn setup(
        &self,
        dot_movement : DotMovement,
        config : m1_da_light_node_util::Config,
    ) -> Result<m1_da_light_node_util::Config, anyhow::Error> {

        // Placeholder for returning the actual configuration.
        Ok(config)
    }
}
