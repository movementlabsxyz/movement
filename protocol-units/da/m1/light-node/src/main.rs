use m1_da_light_node::v1::{
    LightNodeV1,
    LightNodeV1Operations
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    println!("Working?");

    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
                            .unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let light_node = LightNodeV1::try_from_env().await?;

    // log out the node's configuration with tracing
    tracing::info!("{:?}", light_node);
    println!("{:?}", light_node);

    light_node.run().await?;

    Ok(())
}