#![forbid(unsafe_code)]

use clap::*;
use suzuka_util::SuzukaUtil;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	let suzuka_util = SuzukaUtil::parse();

	suzuka_util.execute().await?;

	Ok(())
}
