#![forbid(unsafe_code)]

use clap::*;
use movement_full_node::MovementFullNode;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let tracing_config = movement_tracing::Config::with_metrics_addr("0.0.0.0:9464");
    let _guard = movement_tracing::init_tracing_subscriber(tracing_config);

    let suzuka_util = MovementFullNode::parse();
    suzuka_util.execute().await?;

    Ok(())
}

