use crate::common_args::MovementArgs;
use anyhow::Context;
use clap::Parser;
use movement_da_light_node_client::MovementDaLightNodeClient;
use movement_da_light_node_proto::{blob_response, StreamReadFromHeightRequest};
use tokio_stream::StreamExt;

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
		// Get the config

		let mut client = MovementDaLightNodeClient::try_http2(self.light_node_url.as_str())
			.await
			.context("Failed to connect to light node")?;

		let mut blocks_from_da = client
			.stream_read_from_height(StreamReadFromHeightRequest { height: self.from_height })
			.await
			.context("Failed to stream blocks from DA")?;

		println!("Streaming blocks from DA");

		while let Some(block_res) = blocks_from_da.next().await {
			let response = block_res.context("Failed to get block")?;
			let (_block_bytes, block_timestamp, block_id, da_height) = match response
				.blob
				.ok_or(anyhow::anyhow!("No blob in response"))?
				.blob_type
				.ok_or(anyhow::anyhow!("No blob type in response"))?
			{
				blob_response::BlobType::SequencedBlobBlock(blob) => {
					(blob.data, blob.timestamp, blob.blob_id, blob.height)
				}
				_ => {
					anyhow::bail!("Invalid blob type in response")
				}
			};
			println!("{} {}  {}", hex::encode(block_id), block_timestamp, da_height);
		}

		println!("Finished streaming blocks from DA");

		Ok(())
	}
}
