mod stream_blocks;
mod submit_batch;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for intereacting with the DA")]
pub enum Da {
	StreamBlocks(stream_blocks::StreamBlocks),
	SubmitBatch(submit_batch::SubmitBatch),
}

impl Da {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Da::StreamBlocks(stream_blocks) => stream_blocks.execute().await,
			Da::SubmitBatch(submit_batch) => submit_batch.execute().await,
		}
	}
}
