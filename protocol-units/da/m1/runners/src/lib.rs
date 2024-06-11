pub mod celestia;

pub trait Runner {
    fn run(
        &self, 
        dot_movement : dot_movement::DotMovement,
        config : m1_da_light_node_util::Config,
    ) -> Result<(), anyhow::Error>;
}