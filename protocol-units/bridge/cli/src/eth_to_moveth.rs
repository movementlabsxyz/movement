use crate::clap::eth_to_movement::{Commands, EthSharedArgs};
use alloy::primitives::keccak256;
use anyhow::Result;
use bridge_shared::types::{Amount, HashLock, HashLockPreImage, RecipientAddress, TimeLock};
use bridge_shared::{bridge_contracts::BridgeContractInitiator, types::InitiatorAddress};
use ethereum_bridge::{types::EthAddress, EthClient};
use movement_bridge::utils::MovementAddress;

pub async fn execute(command: &Commands) -> Result<()> {
	match command {
		Commands::Swap { args, recipient, amount } => initiate_swap(args, recipient, *amount).await,
		Commands::Resume { args, transfer_id } => resume_swap(args, transfer_id).await,
	}
}

async fn initiate_swap(
	args: &EthSharedArgs,
	recipient: &MovementAddress,
	amount: u64,
) -> Result<()> {
	println!("Initiating swap to {:?} with amount {}", recipient, amount);

	let mut client = EthClient::new(args).await?;

	// Get the current block height
	let current_block = client.get_block_number().await?;
	println!("Current Ethereum block height: {}", current_block);

	// Convert signer's private key to EthAddress
	let initiator_address = EthAddress(client.get_signer_address());
	let recipient_address = RecipientAddress(From::from(recipient));
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let time_lock = TimeLock(current_block + 100); // Set an appropriate time lock
	let amount = Amount(amount);

	// TODO: Store the swap details in the local database so they can be resumed in case of failure

	client
		.initiate_bridge_transfer(
			InitiatorAddress(initiator_address),
			recipient_address,
			hash_lock,
			time_lock,
			amount,
		)
		.await?;

	// Now we need to listen to the blockchain to receive the correct events and match them accordingly.

	// TODO: I need the bridge transfer ID here to store the state of the swap. Therefore,
	// the initiate bridge transfer function needs to be updated.

	println!("Swap initiated successfully");

	Ok(())
}

async fn resume_swap(args: &EthSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Resuming transfer with ID: {}", transfer_id);

	Ok(())
}
