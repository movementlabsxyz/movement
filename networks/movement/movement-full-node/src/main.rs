#![forbid(unsafe_code)]

use clap::*;
use movement_full_node::MovementFullNode;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let suzuka_util = MovementFullNode::parse();
    let result = suzuka_util.execute().await;

    result
}

