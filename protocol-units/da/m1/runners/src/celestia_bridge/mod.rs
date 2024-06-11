pub mod local;
use crate::Runner;

#[derive(Debug, Clone)]
pub enum CelestiaBridge {
    Local(local::Local),
}

impl CelestiaBridge {
    pub fn local() -> Self {
        CelestiaBridge::Local(local::Local::new())
    }
}

impl Runner for CelestiaBridge {
    async fn run(
        &self, 
        dot_movement : dot_movement::DotMovement,
        config : m1_da_light_node_util::Config,
    ) -> Result<(), anyhow::Error> {
        
        match self {
            CelestiaBridge::Local(local) => {
                local.run(
                    dot_movement,
                    config
                ).await
            }
        }

    }

}