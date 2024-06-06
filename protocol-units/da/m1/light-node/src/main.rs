use m1_da_light_node::v1::{LightNodeV1, LightNodeV1Operations};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: set up tracing-subscriber if the "logging" feature is enabled

    let light_node = LightNodeV1::try_from_env().await?;
    light_node.run().await?;

    Ok(())
}
