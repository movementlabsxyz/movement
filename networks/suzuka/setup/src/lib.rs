pub mod local;

pub trait SuzukaFullNodeSetupOperations {

    async fn setup(
        &self,
        dot_movement : dot_movement::DotMovement,
        config : suzuka_config::Config
    ) -> Result<suzuka_config::Config, anyhow::Error>;

}