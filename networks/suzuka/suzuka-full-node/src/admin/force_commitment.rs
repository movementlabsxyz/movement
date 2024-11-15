use crate::common_args::MovementArgs;
use crate::node::partial::SuzukaPartialNode;
use clap::Parser;
use syncup::SyncupOperations;

#[derive(Debug, Parser, Clone)]
#[clap(
	rename_all = "kebab-case",
	about = "Deletes the resource used for syncing across syncer ids"
)]
pub struct ForceCommitment {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub height: Option<u64>,
}

impl ForceCommitment {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let config = self.movement_args.config().await?;
		let node = SuzukaPartialNode::try_from_config(config)
			.await
			.context("Failed to create the executor")?;

		let height = match self.height {
			Some(height) => height,
			None => node.executor.get_latest_height().await,
		};

		node.executor.revert_block_head_to(height).await?;
		let commitment = node.executor.get_commitment_for_height(height).await?;
		node.settlement_manager().force_attestation(commitment).await?;

		Ok(())
	}
}
