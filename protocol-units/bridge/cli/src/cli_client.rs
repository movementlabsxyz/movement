use crate::subcommands::{EthSubCommands, MovementSubCommands};
use anyhow::Result;
use bridge_service::{
	chains::{
		bridge_contracts::BridgeContract,
		ethereum::{client::EthClient, types::EthAddress},
		movement::{client::MovementClient, utils::MovementAddress},
	},
	types::{
		Amount, AssetType, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock,
		HashLockPreImage,
	},
};

pub struct Client {
	eth: EthClient,
	movement: MovementClient,
}

impl Client {
	// Accepting clients by value instead of references.
	pub fn new(eth_client: EthClient, movement_client: MovementClient) -> Self {
		Self { eth: eth_client, movement: movement_client }
	}

	pub async fn eth_movement_execute(&self, command: &EthSubCommands) -> Result<()> {
		match command {
			EthSubCommands::Initiate { recipient, amount, hash_lock } => {
				let hash_lock = HashLock(
					hex::decode(hash_lock)
						.expect("Invalid hex for hash lock")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for hash lock"))?,
				);
				let recipient_address = BridgeAddress(recipient.0.clone());
				self.eth
					.initiate_bridge_transfer(
						BridgeAddress(EthAddress(self.eth.get_signer_address())),
						recipient_address,
						hash_lock,
						Amount(AssetType::Moveth(amount.clone())),
					)
					.await?;
				println!("Bridge transfer initiated successfully.");
			}

			EthSubCommands::Complete { transfer_id, preimage } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);
				let pre_image = HashLockPreImage(
					hex::decode(preimage).expect("Invalid hex for pre-image").try_into().unwrap(),
				);

				self.eth.initiator_complete_bridge_transfer(transfer_id, pre_image).await?;
				println!("Bridge transfer completed successfully.");
			}

			EthSubCommands::Refund { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);

				self.eth.refund_bridge_transfer(transfer_id).await?;
				println!("Bridge transfer refunded successfully.");
			}

			EthSubCommands::Details { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);

				let details: Option<BridgeTransferDetails<EthAddress>> =
					self.eth.get_bridge_transfer_details_initiator(transfer_id).await?;
				match details {
					Some(details) => {
						println!("Initiator Details: {:?}", details);
					}
					None => {
						println!("No details found for the transfer.");
					}
				}
			}
		}

		Ok(())
	}

	pub async fn movement_eth_execute(&self, command: &MovementSubCommands) -> Result<()> {
		match command {
			MovementSubCommands::Initiate { recipient, amount, hash_lock } => {
				let hash_lock = HashLock(
					hex::decode(hash_lock)
						.expect("Invalid hex for hash lock")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for hash lock"))?,
				);
				let recipient_address = BridgeAddress(recipient.0.clone());
				self.movement
					.initiate_bridge_transfer(
						BridgeAddress(MovementAddress(self.movement.native_address)),
						recipient_address,
						hash_lock,
						Amount(AssetType::Moveth(*amount)),
					)
					.await?;
				println!("Bridge transfer initiated successfully.");
			}

			MovementSubCommands::Complete { transfer_id, pre_image } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);
				let pre_image = HashLockPreImage(
					hex::decode(pre_image).expect("Invalid hex for pre-image").try_into().unwrap(),
				);

				self.movement.initiator_complete_bridge_transfer(transfer_id, pre_image).await?;
				println!("Bridge transfer completed successfully.");
			}

			MovementSubCommands::Refund { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);

				self.movement.refund_bridge_transfer(transfer_id).await?;
				println!("Bridge transfer refunded successfully.");
			}

			MovementSubCommands::Lock { transfer_id, initiator, recipient, amount, hash_lock } => {
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
						.map_err(|_| anyhow::anyhow!("Invalid hex for initiator address"))?,
				);
				let recipient_address = recipient.clone();
				self.movement
					.lock_bridge_transfer(
						transfer_id,
						hash_lock,
						initiator_address,
						recipient_address,
						Amount(AssetType::Moveth(*amount)),
					)
					.await?;
				println!("Bridge transfer locked successfully.");
			}

			// Handling the missing variants
			MovementSubCommands::Abort { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);

				self.movement.abort_bridge_transfer(transfer_id).await?;
				println!("Bridge transfer aborted successfully.");
			}

			MovementSubCommands::DetailsInitiator { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);

				let details =
					self.movement.get_bridge_transfer_details_initiator(transfer_id).await?;

				match details {
					Some(details) => {
						println!("Initiator Details: {:?}", details);
					}
					None => {
						println!("No details found for the transfer.");
					}
				}
			}

			MovementSubCommands::DetailsCounterparty { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);

				let details =
					self.movement.get_bridge_transfer_details_counterparty(transfer_id).await?;

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
}
