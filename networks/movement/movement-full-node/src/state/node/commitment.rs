use crate::common_args::MovementArgs;
use crate::node::partial::MovementPartialNode;
use anyhow::Context;
use clap::Parser;
use maptos_dof_execution::DynOptFinExecutor;
use tracing::info;

#[derive(Debug, Parser, Clone)]
#[clap(
	rename_all = "kebab-case",
	about = "Gets the block commitment that the node would make at a certain height. v0 ignores super block logic."
)]
pub struct Commitment {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub height: Option<u64>,
}

impl Commitment {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		info!("Forcing commitment");
		let config = self.movement_args.config().await?;
		info!("Loaded config {:?}", config);
		let executor = MovementPartialNode::try_executor_from_config(config)
			.await
			.context("Failed to create the executor")?;

		let height = match self.height {
			Some(height) => height,
			None => executor.get_block_head_height()?,
		};

		let commitment = executor.get_block_commitment_by_height(height).await?;
		// Use println as this is standard (non-logging output)
		println!("{:?}", Some(commitment));

		Ok(())
	}
}
