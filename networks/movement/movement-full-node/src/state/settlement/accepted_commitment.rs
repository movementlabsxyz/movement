use crate::common_args::MovementArgs;
use crate::node::partial::MovementPartialNode;
use anyhow::Context;
use clap::Parser;
use maptos_dof_execution::DynOptFinExecutor;
use mcr_settlement_client::{McrSettlementClient, McrSettlementClientOperations};
use tracing::info;

#[derive(Debug, Parser, Clone)]
#[clap(
	rename_all = "kebab-case",
	about = "Gets the last accepted commitment matching the height of this node or a provided height in the designated settlement contract."
)]
pub struct AcceptedCommitment {
	#[clap(flatten)]
	pub movement_args: MovementArgs,
	pub height: Option<u64>,
}

impl AcceptedCommitment {
	pub async fn execute(&self) -> Result<(), anyhow::Error> {
		info!("Forcing commitment");
		let config = self.movement_args.config().await?;
		info!("Loaded config {:?}", config);
		let settlement_client = McrSettlementClient::build_with_config(&config.mcr)
			.await
			.context("Failed to build MCR settlement client with config")?;
		info!("Built settlement client");
		let executor = MovementPartialNode::try_executor_from_config(config)
			.await
			.context("Failed to create the executor")?;

		let height = match self.height {
			Some(height) => height,
			None => executor.get_block_head_height()?,
		};

		let commitment = settlement_client.get_commitment_at_height(height).await?;
		// Use println as this is standard (non-logging output)
		match commitment {
			Some(commitment) => println!("{}", commitment),
			None => println!("No commitment found at height {}", height),
		}

		Ok(())
	}
}
