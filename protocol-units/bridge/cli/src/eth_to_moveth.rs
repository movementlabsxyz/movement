use crate::clap::eth_to_movement::{Commands, EthSharedArgs};
use alloy::primitives::keccak256;
use anyhow::Result;
use bridge_shared::bridge_contracts::BridgeContractInitiator;
use bridge_shared::types::{
	Amount, AssetType, HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress,
};
use ethereum_bridge::{client::EthClient, types::EthAddress};
use movement_bridge::utils::MovementAddress;

pub async fn execute(command: &Commands) -> Result<()> {
	match command {
		Commands::Initiate { args, recipient, amount } => {
			initiate_transfer(args, recipient, *amount).await
		}
		Commands::Complete { args, transfer_id } => complete_transfer(args, transfer_id).await,
	}
}

async fn initiate_transfer(
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
	let amount = Amount(AssetType::EthAndWeth((amount, 0)));

	client
		.initiate_bridge_transfer(
			InitiatorAddress(initiator_address),
			recipient_address,
			hash_lock,
			amount,
		)
		.await?;

	Ok(())
}

async fn complete_transfer(args: &EthSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Resuming transfer with ID: {}", transfer_id);

	Ok(())
}
