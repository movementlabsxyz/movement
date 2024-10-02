#![allow(unused_imports)]
use crate::clap::eth_to_movement::EthSharedArgs;
use alloy::primitives::keccak256;
use anyhow::Result;
use bridge_service::{
	chains::{
		bridge_contracts::BridgeContract,
		ethereum::{
			client::{Config, EthClient},
			types::EthAddress,
		},
		movement::utils::MovementAddress,
	},
	types::{
		Amount, AssetType, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock,
		HashLockPreImage,
	},
};
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
	Initiate {
		recipient: BridgeAddress<Vec<u8>>,
		amount: u64,
		hash_lock: String,
	},
	Complete {
		transfer_id: String,
		pre_image: String,
	},
	Refund {
		transfer_id: String,
	},
	Lock {
		transfer_id: String,
		initiator: String,
		recipient: BridgeAddress<Vec<u8>>,
		amount: u64,
		hash_lock: String,
	},
	Abort {
		transfer_id: String,
	},
	DetailsInitiator {
		transfer_id: String,
	},
	DetailsCounterparty {
		transfer_id: String,
	},
}

pub async fn execute(command: &Commands, client: &EthClient) -> Result<()> {
	match command {
		Commands::Initiate { recipient, amount, hash_lock } => {
			let hash_lock = HashLock(
				hex::decode(hash_lock).expect("Invalid hex for hash lock").try_into().unwrap(),
			);

			client
				.initiate_bridge_transfer(
					BridgeAddress(EthAddress(client.get_signer_address())),
					recipient.clone(),
					hash_lock,
					Amount(AssetType::Moveth(*amount)),
				)
				.await?;
			println!("Bridge transfer initiated successfully.");
		}

		Commands::Complete { transfer_id, pre_image } => {
			let transfer_id = BridgeTransferId(
				hex::decode(transfer_id)
					.expect("Invalid hex for transfer ID")
					.try_into()
					.unwrap(),
			);
			let pre_image = HashLockPreImage(
				hex::decode(pre_image).expect("Invalid hex for pre-image").try_into().unwrap(),
			);

			client.initiator_complete_bridge_transfer(transfer_id, pre_image).await?;
			println!("Bridge transfer completed successfully.");
		}

		Commands::Refund { transfer_id } => {
			let transfer_id = BridgeTransferId(
				hex::decode(transfer_id)
					.expect("Invalid hex for transfer ID")
					.try_into()
					.unwrap(),
			);

			client.refund_bridge_transfer(transfer_id).await?;
			println!("Bridge transfer refunded successfully.");
		}

		Commands::Lock { transfer_id, initiator, recipient, amount, hash_lock } => {
			let transfer_id = BridgeTransferId(
				hex::decode(transfer_id)
					.expect("Invalid hex for transfer ID")
					.try_into()
					.unwrap(),
			);
			let hash_lock = HashLock(
				hex::decode(hash_lock).expect("Invalid hex for hash lock").try_into().unwrap(),
			);
			let initiator_address = BridgeAddress(
				hex::decode(initiator)
					.expect("Invalid hex for initiator address")
					.try_into()
					.unwrap(),
			);

			client
				.lock_bridge_transfer(
					transfer_id,
					hash_lock,
					initiator_address,
					recipient.clone(),
					Amount(AssetType::Moveth(*amount)),
				)
				.await?;
			println!("Bridge transfer locked successfully.");
		}

		Commands::Abort { transfer_id } => {
			let transfer_id = BridgeTransferId(
				hex::decode(transfer_id)
					.expect("Invalid hex for transfer ID")
					.try_into()
					.unwrap(),
			);

			client.abort_bridge_transfer(transfer_id).await?;
			println!("Bridge transfer aborted successfully.");
		}

		Commands::DetailsInitiator { transfer_id } => {
			let transfer_id = BridgeTransferId(
				hex::decode(transfer_id)
					.expect("Invalid hex for transfer ID")
					.try_into()
					.unwrap(),
			);

			let details: Option<BridgeTransferDetails<EthAddress>> =
				client.get_bridge_transfer_details_initiator(transfer_id).await?;
			match details {
				Some(details) => {
					println!("Initiator Details: {:?}", details);
				}
				None => {
					println!("No details found for the transfer.");
				}
			}
		}

		Commands::DetailsCounterparty { transfer_id } => {
			let transfer_id = BridgeTransferId(
				hex::decode(transfer_id)
					.expect("Invalid hex for transfer ID")
					.try_into()
					.unwrap(),
			);

			let details = client.get_bridge_transfer_details_counterparty(transfer_id).await?;
			match details {
				Some(details) => {
					println!("Counterparty Details: {:?}", details);
				}
				None => {
					println!("No details found for the transfer.");
				}
			}
		}
	}

	Ok(())
}
