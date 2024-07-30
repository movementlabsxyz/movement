use crate::clap::eth_to_movement::{Commands, EthSharedArgs};
use anyhow::Result;
use ethereum_bridge::EthClient;

pub async fn execute(command: &Commands) -> Result<()> {
	match command {
		Commands::Swap { args, recipient, amount } => initiate_swap(args, recipient, *amount).await,
		Commands::Resume { args, transfer_id } => resume_swap(args, transfer_id).await,
	}
}

async fn initiate_swap(args: &EthSharedArgs, recipient: &str, amount: u64) -> Result<()> {
	println!("Initiating swap to {} with amount {}", recipient, amount);

	let client = EthClient::new(args).await?;

	Ok(())
}

async fn resume_swap(args: &EthSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Resuming transfer with ID: {}", transfer_id);

	Ok(())
}
