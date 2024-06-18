use m1_da_light_node::v1::{LightNodeV1, LightNodeV1Operations};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// TODO: set up tracing-subscriber if the "logging" feature is enabled

    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // get the dot movement from the environment
    let dot_movement = dot_movement::DotMovement::try_from_env()?;
    // get the path to the configuration file
    let path = dot_movement.get_path().join("config.toml");
    // get the configuration from the configuration file
    let config = m1_da_light_node_util::Config::try_from_toml_file(&path).unwrap_or_default();
    
    let light_node = LightNodeV1::try_from_config(config).await?;

    // log out the node's configuration with tracing
    tracing::info!("{:?}", light_node);

    light_node.run().await?;

    Ok(())
}
