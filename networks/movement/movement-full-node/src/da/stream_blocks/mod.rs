use crate::common_args::MovementArgs;
use anyhow::Context;
use clap::Parser;
use movement_da_sequencer_client::DaSequencerClient;
use movement_da_sequencer_client::GrpcDaSequencerClient;
use movement_da_sequencer_proto::StreamReadFromHeightRequest;
use tokio_stream::StreamExt;
use tracing::info;
use url::Url;

#[derive(Debug, Parser, Clone)]
#[clap(rename_all = "kebab-case", about = "Streams the DA blocks")]
pub struct StreamBlocks {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub light_node_url: String,
	pub from_height: u64,
}

impl StreamBlocks {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let mut client = GrpcDaSequencerClient::try_connect(
			&Url::parse(&self.light_node_url).expect("Can't parse provided url."),
			10,
		)
		.await
		.expect("gRPC client connection failed.");

		let (mut blocks_from_da, _aleert_channel) = client
			.stream_read_from_height(StreamReadFromHeightRequest { height: self.from_height })
			.await
			.context("Failed to stream blocks from DA")?;

		info!("streaming blocks from DA");

		while let Some(block_res) = blocks_from_da.next().await {
			let block = block_res.context("Failed to get block")?;
			// pretty print (with labels) the block_id, block_timestamp, and da_height
			tracing::info!(
				"Block ID: {}, DA Height: {}",
				hex::encode(block.block_id),
				block.height
			);
		}

		info!("Finished streaming blocks from DA");

		Ok(())
	}
}
