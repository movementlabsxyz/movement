use crate::common_args::MovementArgs;
use anyhow::Context;
use clap::Parser;
use movement_da_light_node_client::MovementDaLightNodeClient;
use movement_da_light_node_proto::{blob_response, StreamReadFromHeightRequest};
use tokio_stream::StreamExt;
use tracing::info;

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

		let mut client = MovementDaLightNodeClient::try_http1(self.light_node_url.as_str())
			.context("Failed to connect to light node")?;

		let mut blocks_from_da = client
			.stream_read_from_height(StreamReadFromHeightRequest { height: self.from_height })
			.await
			.context("Failed to stream blocks from DA")?;

		info!("streaming blocks from DA");

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
				blob_response::BlobType::PassedThroughBlob(blob) => {
					(blob.data, blob.timestamp, blob.blob_id, blob.height)
				}
				blob_response::BlobType::HeartbeatBlob(_) => {
					tracing::info!("Receive heartbeat blob");
					continue;
				}
				_ => {
					anyhow::bail!("Invalid blob type in response")
				}
			};

			// pretty print (with labels) the block_id, block_timestamp, and da_height
			tracing::info!(
				"Block ID: {}, Block Timestamp: {}, DA Height: {}",
				hex::encode(block_id),
				// unix date string from the block timestamp which is in microseconds
				chrono::DateTime::from_timestamp((block_timestamp / 1_000_000) as i64, 0)
					.context("Failed to convert timestamp to date")?,
				da_height
			);
		}

		Ok(())
	}
}
