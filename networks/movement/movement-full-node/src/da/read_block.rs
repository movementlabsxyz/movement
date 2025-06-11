use crate::common_args::MovementArgs;
use anyhow::Context;
use clap::Parser;
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_client::GrpcDaSequencerClient;
use tracing::info;
use url::Url;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Allow to get a block at specified height.")]
pub struct ReadBlock {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub light_node_url: String,
	pub height: u64,
}

impl ReadBlock {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let mut client = GrpcDaSequencerClient::try_connect(
			&Url::parse(&self.light_node_url).expect("Can't parse provided url."),
			10,
		)
		.await
		.expect("gRPC client connection failed.");

		let blocks = client
			.read_at_height(self.height)
			.await
			.context("Failed to read_at_height from DA")?;

		info!("Block read at height:{} : {blocks:?}", self.height);
		Ok(())
	}
}
