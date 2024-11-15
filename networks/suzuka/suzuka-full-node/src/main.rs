#![forbid(unsafe_code)]

use clap::*;
use suzuka_full_node::SuzukaFullNode;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let suzuka_util = SuzukaFullNode::parse();

	suzuka_util.execute().await?;

	Ok(())
}
