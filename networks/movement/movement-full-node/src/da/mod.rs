mod run;
pub mod stream_blocks;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case", about = "Commands for intereacting with the DA")]
pub enum Da {
	StreamBlocks(stream_blocks::StreamBlocks),
	Run(run::DaRun),
}

impl Da {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		match self {
			Da::StreamBlocks(stream_blocks) => stream_blocks.execute().await,
			Da::Run(da) => da.execute().await,
		}
	}
}
