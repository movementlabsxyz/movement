pub mod local;

pub trait MonzaFullNodeSetupOperations {

    async fn setup(
        &self,
        dot_movement : dot_movement::DotMovement,
        config : suzuka_config::Config
    ) -> Result<suzuka_config::Config, anyhow::Error>;

}