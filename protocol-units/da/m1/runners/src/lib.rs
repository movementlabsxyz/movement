pub mod celestia_appd;
pub mod celestia_bridge;

pub trait Runner {
    async fn run(
        &self, 
        dot_movement : dot_movement::DotMovement,
        config : m1_da_light_node_util::Config,
    ) -> Result<(), anyhow::Error>;
}