use crate::common_args::MovementArgs;
use crate::node::partial::SuzukaPartialNode;
use anyhow::Context;
use clap::Parser;
use maptos_dof_execution::DynOptFinExecutor;
use mcr_settlement_client::{McrSettlementClient, McrSettlementClientOperations};
use tracing::info;

#[derive(Debug, Parser, Clone)]
#[clap(
	rename_all = "kebab-case",
	about = "Forces the accepted commitment of MCR by height. If no height is provided, uses the latest height on this node."
)]
pub struct ForceCommitment {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub height: Option<u64>,
}

impl ForceCommitment {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		info!("Forcing commitment");
		let config = self.movement_args.config().await?;
		info!("Loaded config {:?}", config);
		let settlement_client = McrSettlementClient::build_with_config(&config.mcr)
			.await
			.context("Failed to build MCR settlement client with config")?;
		info!("Built settlement client");
		let executor = SuzukaPartialNode::try_executor_from_config(config)
			.await
			.context("Failed to create the executor")?;

		let height = match self.height {
			Some(height) => height,
			None => executor.get_block_head_height()?,
		};
		info!("Forcing commmitment at height {}", height);
		executor.revert_block_head_to(height).await?;
		let commitment = executor.get_block_commitment_by_height(height).await?;

		settlement_client.force_block_commitment(commitment).await?;

		Ok(())
	}
}
