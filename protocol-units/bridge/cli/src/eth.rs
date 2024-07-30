use crate::{
	clap::{EthCommands, EthShared},
	state::{load_swap_state, save_swap_state, SwapStatus},
};
use anyhow::Result;
use uuid::Uuid;

pub async fn command(command: &EthCommands) -> Result<()> {
	match command {
		EthCommands::Swap { args, recipient, amount } => {
			initiate_swap(args, recipient, *amount).await
		}
		EthCommands::Resume { args, transfer_id } => resume_swap(args, transfer_id).await,
	}
}

async fn initiate_swap(args: &EthShared, recipient: &str, amount: u64) -> Result<()> {
	println!("Initiating swap to {} with amount {}", recipient, amount);

	// Create a new swap state
	let swap_state = crate::state::SwapState {
		id: Uuid::new_v4().to_string(),
		recipient: recipient.to_string(),
		amount,
		status: SwapStatus::Initiated,
	};

	// Save the initial state
	save_swap_state(&swap_state)?;

	// Implement the actual swap initiation logic here
	// For now, we'll just print a message
	println!("Swap initiated with ID: {}", swap_state.id);

	Ok(())
}

async fn resume_swap(args: &EthShared, transfer_id: &str) -> Result<()> {
	println!("Resuming transfer with ID: {}", transfer_id);

	let _swap_state = load_swap_state(transfer_id)?;

	//

	Ok(())
}
