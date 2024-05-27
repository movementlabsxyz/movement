use anyhow::Context;
use suzuka_full_node::{
    SuzukaNode,
    partial::SuzukaPartialNode,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

    #[cfg(feature = "logging")]
    {
        use tracing_subscriber::EnvFilter;

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env()
                             .unwrap_or_else(|_| EnvFilter::new("info")))
            .init();

    }

    let executor = SuzukaPartialNode::try_from_env().await.context(
        "Failed to create the executor"
    )?;

    executor.run().await.context(
        "Failed to run the executor"
    )?;

    Ok(())

}
