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
}

impl ForceCommitment {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		let config = self.movement_args.config().await?;

		let node = SuzukaPartialNode::try_from_config(config)
			.await
			.context("Failed to create the executor")?;
		node.executor.revert_block_head_to(height);
		commit = node.executor.get_latest_commitment();
		node.settlement_manager().forceAttestation(commitment);

		Ok(())
	}
}
