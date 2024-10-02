use crate::subcommands::TransferSubcommands;
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
	pub fn new(eth_client: EthClient, movement_client: MovementClient) -> Self {
		Self { eth: eth_client, movement: movement_client }
	}

	pub async fn eth_movement_execute(&self, command: &TransferSubcommands) -> Result<()> {
		match command {
			TransferSubcommands::Initiate { recipient, amount, hash_lock } => {
				let hash_lock = HashLock(
					hex::decode(hash_lock)
						.expect("Invalid hex for hash lock")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for hash lock"))?,
				);

				// Remove the "0x" prefix if it's present in the recipient address
				let recipient = recipient.strip_prefix("0x").unwrap_or(recipient);

				// Decode the recipient address from hex string to Vec<u8>
				let recipient_bytes: Vec<u8> =
					hex::decode(recipient).expect("Invalid hex for recipient address");
				if recipient_bytes.len() != 32 {
					return Err(anyhow::anyhow!("Recipient address must be 32 bytes"));
				}
				self.eth
					.initiate_bridge_transfer(
						BridgeAddress(EthAddress(self.eth.get_signer_address())),
						BridgeAddress(recipient_bytes),
						hash_lock,
						Amount(AssetType::Moveth(amount.clone())),
					)
					.await?;
				tracing::info!("Bridge transfer initiated successfully.");
			}
			TransferSubcommands::Complete { transfer_id, preimage } => {
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
				tracing::info!("Bridge transfer completed successfully.");
			}
			TransferSubcommands::Details { transfer_id } => {
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
						tracing::info!("Initiator Details: {:?}", details);
					}
					None => {
						tracing::info!("No details found for the transfer.");
					}
				}
			}
			TransferSubcommands::InitiatorRefund { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);
				self.movement.refund_bridge_transfer(transfer_id).await?;
				tracing::info!("Bridge transfer refunded successfully.");
			}
			TransferSubcommands::CounterpartyAbort { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);
				self.movement.abort_bridge_transfer(transfer_id).await?;
				tracing::info!("Bridge transfer aborted successfully.");
			}
		}

		Ok(())
	}

	pub async fn movement_eth_execute(&self, command: &TransferSubcommands) -> Result<()> {
		match command {
			TransferSubcommands::Initiate { recipient, amount, hash_lock } => {
				let hash_lock = HashLock(
					hex::decode(hash_lock)
						.expect("Invalid hex for hash lock")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for hash lock"))?,
				);

				let recipient = recipient.strip_prefix("0x").unwrap_or(recipient);
				let recipient_bytes: Vec<u8> =
					hex::decode(recipient).expect("Invalid hex for recipient address");
				if recipient_bytes.len() != 32 {
					return Err(anyhow::anyhow!("Recipient address must be 32 bytes"));
				}
				self.movement
					.initiate_bridge_transfer(
						BridgeAddress(MovementAddress(self.movement.native_address)),
						BridgeAddress(recipient_bytes),
						hash_lock,
						Amount(AssetType::Moveth(*amount)),
					)
					.await?;
				tracing::info!("Bridge transfer initiated successfully.");
			}
			TransferSubcommands::Complete { transfer_id, preimage } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);
				let pre_image = HashLockPreImage(
					hex::decode(preimage).expect("Invalid hex for pre-image").try_into().unwrap(),
				);
				self.movement.initiator_complete_bridge_transfer(transfer_id, pre_image).await?;
				tracing::info!("Bridge transfer completed successfully.");
			}
			TransferSubcommands::InitiatorRefund { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);
				self.movement.refund_bridge_transfer(transfer_id).await?;
				tracing::info!("Bridge transfer refunded successfully.");
			}
			TransferSubcommands::CounterpartyAbort { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.map_err(|_| anyhow::anyhow!("Invalid hex for transfer ID"))?,
				);
				self.movement.abort_bridge_transfer(transfer_id).await?;
				tracing::info!("Bridge transfer aborted successfully.");
			}
			TransferSubcommands::Details { transfer_id } => {
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
						tracing::info!("Counterparty Details: {:?}", details);
					}
					None => {
						tracing::info!("No details found for the transfer.");
					}
				}
			}
		}

		Ok(())
	}
}
