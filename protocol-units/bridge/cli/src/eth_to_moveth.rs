use crate::clap::eth_to_movement::{Commands, EthSharedArgs, MoveSharedArgs};
use alloy::primitives::keccak256;
use anyhow::Result;
use bridge_shared::bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator};
use bridge_shared::types::{
	Amount, AssetType, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress,
	RecipientAddress, TimeLock,
};
use ethereum_bridge::{client::EthClient, types::EthAddress};
use movement_bridge::utils::MovementAddress;
use movement_bridge::MovementClient;
use std::convert::TryInto;

pub async fn execute(command: &Commands) -> Result<()> {
	match command {
		Commands::IniatializeUser { args } => Ok(()),
		Commands::FromEthereum { args, recipient, amount } => {
			bridge_initiator_eth(args, recipient, *amount).await
		}
		Commands::FromMovement { args, recipient, amount } => {
			bridge_initiator_move(args, recipient, *amount).await
		}
		Commands::LockOnEthereum { args, amount, originator, recipient, transfer_id } => {
			lock_counterparty_eth(args, *amount, originator, recipient, transfer_id).await
		}
		Commands::LockOnMovement { args, amount, originator, recipient, transfer_id } => {
			lock_counterparty_move(args, *amount, originator, recipient, transfer_id).await
		}
		Commands::CompleteInitiatorOnEthereum { args, transfer_id } => {
			complete_initiator_eth(args, transfer_id).await
		}
		Commands::CompleteInitiatorOnMovement { args, transfer_id } => {
			complete_initiator_move(args, transfer_id).await
		}
		Commands::CompleteCounterpartyOnEthereum { args, transfer_id } => {
			complete_counterparty_eth(args, transfer_id).await
		}
		Commands::CompleteCounterpartyOnMovement { args, transfer_id } => {
			complete_counterparty_move(args, transfer_id).await
		}
		Commands::CancelOnEthereum { args, transfer_id } => {
			cancel_counterparty_eth(args, transfer_id).await
		}
		Commands::CancelOnMovement { args, transfer_id } => {
			cancel_counterparty_move(args, transfer_id).await
		}
		Commands::RefundOnEthereum { args, transfer_id } => {
			refund_counterparty_eth(args, transfer_id).await
		}
		Commands::RefundOnMovement { args, transfer_id } => {
			refund_counterparty_move(args, transfer_id).await
		}
	}
}

async fn bridge_initiator_eth(
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
	let originator_address = EthAddress(client.get_signer_address());
	let recipient_address = RecipientAddress(From::from(recipient));
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let time_lock = TimeLock(current_block + 100); // Set an appropriate time lock
	let amount = Amount(AssetType::EthAndWeth((amount, 0)));

	// Call using rust based eth libs
	client
		.initiate_bridge_transfer(
			InitiatorAddress(originator_address),
			recipient_address,
			hash_lock,
			time_lock,
			amount,
		)
		.await?;
	Ok(())
}

async fn bridge_initiator_move(
	args: &MoveSharedArgs,
	recipient: &EthAddress,
	amount: u64,
) -> Result<()> {
	println!("Initiating swap to {:?} with amount {}", recipient, amount);

	// let mut client = MovementClient::new(args).await?;

	// Get the current block height
	// let current_block = client.get_block_number().await?;
	// println!("Current Ethereum block height: {}", current_block);

	// // Convert signer's private key to EthAddress
	// let originator_address = MovementAddress(client.si9().await);
	// let recipient_address: RecipientAddress<Vec<u8>> = RecipientAddress(From::from(recipient.to_vec()));
	// let hash_lock_pre_image = HashLockPreImage::random();
	// let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	// let time_lock = TimeLock(current_block + 100); // Set an appropriate time lock
	// let amount = Amount(AssetType::Moveth(amount));

	// client
	// 	.initiate_bridge_transfer(
	// 		recipient_address,
	// 		hash_lock,
	// 		time_lock,
	// 		amount,
	// 	)
	// 	.await?;

	// Now we need to listen to the blockchain to receive the correct events and match them accordingly.

	// TODO: I need the bridge transfer ID here to store the state of the swap. Therefore,
	// the initiate bridge transfer function needs to be updated.

	println!("Swap initiated successfully");

	Ok(())
}

async fn lock_counterparty_eth(
	args: &EthSharedArgs,
	amount: u64,
	originator: &MovementAddress,
	recipient: &EthAddress,
	transfer_id: &str,
) -> Result<()> {
	println!("Lock transfer with ID: {}", transfer_id);
	let mut client = EthClient::new(args).await?;

	let current_block = client.get_block_number().await?;

	// Convert signer's private key to EthAddress
	let transfer_id_slice: &[u8] = transfer_id.as_bytes();

	let bridge_transfer_id: [u8; 32] =
		transfer_id_slice.try_into().expect("transfer_id should be 32 bytes long");
	let originator_address = InitiatorAddress(originator.to_vec());
	let recipient_address = RecipientAddress(EthAddress(recipient.0));
	let hash_lock_pre_image = HashLockPreImage::random();
	let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	let time_lock = TimeLock(current_block + 100); // Set an appropriate time lock
	let amount = Amount(AssetType::EthAndWeth((0, amount)));

	client
		.lock_bridge_transfer(
			BridgeTransferId(bridge_transfer_id),
			hash_lock,
			time_lock,
			originator_address,
			recipient_address,
			amount,
		)
		.await?;
	Ok(())
}

async fn lock_counterparty_move(
	args: &MoveSharedArgs,
	amount: u64,
	originator: &EthAddress,
	recipient: &MovementAddress,
	transfer_id: &str,
) -> Result<()> {
	println!("Lock transfer with ID: {}", transfer_id);

	let mut client = MovementClient::new(args).await?;

	// // Convert signer's private key to EthAddress
	// let originator_address = InitiatorAddress(client.get_signer_address());
	// let recipient_address: RecipientAddress<Vec<u8>> = RecipientAddress(From::from(recipient.to_vec()));
	// let hash_lock_pre_image = HashLockPreImage::random();
	// let hash_lock = HashLock(From::from(keccak256(hash_lock_pre_image)));
	// let time_lock = TimeLock(current_block + 100); // Set an appropriate time lock
	// let amount = Amount(AssetType::EthAndWeth((0,amount)));

	// client.lock_bridge_transfer(transfer_id, hash_lock).await?;

	Ok(())
}

async fn complete_initiator_eth(args: &EthSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Complete transfer with ID: {}", transfer_id);

	Ok(())
}

async fn complete_initiator_move(args: &MoveSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Complete transfer with ID: {}", transfer_id);

	Ok(())
}

async fn complete_counterparty_eth(args: &EthSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Complete transfer with ID: {}", transfer_id);

	Ok(())
}

async fn complete_counterparty_move(args: &MoveSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Complete transfer with ID: {}", transfer_id);

	Ok(())
}
async fn cancel_counterparty_eth(args: &EthSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Cancel transfer with ID: {}", transfer_id);

	Ok(())
}

async fn cancel_counterparty_move(args: &MoveSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Cancel transfer with ID: {}", transfer_id);

	Ok(())
}
async fn refund_counterparty_eth(args: &EthSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Refund transfer with ID: {}", transfer_id);

	Ok(())
}

async fn refund_counterparty_move(args: &MoveSharedArgs, transfer_id: &str) -> Result<()> {
	println!("Refund transfer with ID: {}", transfer_id);

	Ok(())
}
