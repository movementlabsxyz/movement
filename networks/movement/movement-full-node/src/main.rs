#![forbid(unsafe_code)]

use clap::*;
use movement_full_node::MovementFullNode;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let tracing_config = movement_tracing::Config::with_metrics_addr("127.0.0.1:9464");
    let _guard = movement_tracing::init_telemetry(tracing_config).await;
    
    tokio::time::sleep(Duration::from_secs(1)).await;

    let suzuka_util = MovementFullNode::parse();
    let result = suzuka_util.execute().await;

    result
}

