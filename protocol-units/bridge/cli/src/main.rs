use anyhow::Result;
use bridge_cli::{
	clap::{BridgeCommands, CliOptions},
	eth_to_movement::EthSubCommands,
};
use bridge_service::chains::{
	bridge_contracts::BridgeContract,
	ethereum::client::{Config as EthConfig, EthClient},
	ethereum::types::EthAddress,
	movement::client::{Config as MovementConfig, MovementClient},
};
use bridge_service::types::{
	Amount, AssetType, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock,
	HashLockPreImage,
};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
	inner_main().await.map_err(|e| anyhow::anyhow!(e))
}

async fn inner_main() -> anyhow::Result<()> {
	tracing_subscriber::fmt::init();

	let eth_client = EthClient::new(EthConfig::build_for_test())
		.await
		.expect("Failed to create EthClient");
	let movement_client = MovementClient::new(&MovementConfig::build_for_test())
		.await
		.expect("Failed to create MovementClient");

	let client = Client::new(eth_client, movement_client); // Pass by value, not reference

	let cli = CliOptions::parse();
	match &cli.command {
		BridgeCommands::BridgeEthToMovETH(command) => {
			client.execute(command).await?;
		}
	}

	Ok(())
}

pub struct Client {
	eth: EthClient,
	#[allow(unused)]
	movement: MovementClient,
}

impl Client {
	// Accepting clients by value instead of references.
	fn new(eth_client: EthClient, movement_client: MovementClient) -> Self {
		Self { eth: eth_client, movement: movement_client }
	}

	pub async fn execute(&self, command: &EthSubCommands) -> Result<()> {
		match command {
			EthSubCommands::Initiate { recipient, amount, hash_lock } => {
				let hash_lock = HashLock(
					hex::decode(hash_lock).expect("Invalid hex for hash lock").try_into().unwrap(),
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

			EthSubCommands::Complete { transfer_id, pre_image } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.unwrap(),
				);
				let pre_image = HashLockPreImage(
					hex::decode(pre_image).expect("Invalid hex for pre-image").try_into().unwrap(),
				);

				self.eth.initiator_complete_bridge_transfer(transfer_id, pre_image).await?;
				println!("Bridge transfer completed successfully.");
			}

			EthSubCommands::Refund { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.unwrap(),
				);

				self.eth.refund_bridge_transfer(transfer_id).await?;
				println!("Bridge transfer refunded successfully.");
			}

			EthSubCommands::Lock { transfer_id, initiator, recipient, amount, hash_lock } => {
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
				let recipient_address = BridgeAddress(recipient.0.clone());

				self.eth
					.lock_bridge_transfer(
						transfer_id,
						hash_lock,
						initiator_address,
						recipient_address,
						Amount(AssetType::Moveth(amount.clone())),
					)
					.await?;
				println!("Bridge transfer locked successfully.");
			}

			EthSubCommands::Abort { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.unwrap(),
				);

				self.eth.abort_bridge_transfer(transfer_id).await?;
				println!("Bridge transfer aborted successfully.");
			}

			EthSubCommands::DetailsInitiator { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.unwrap(),
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

			EthSubCommands::DetailsCounterparty { transfer_id } => {
				let transfer_id = BridgeTransferId(
					hex::decode(transfer_id)
						.expect("Invalid hex for transfer ID")
						.try_into()
						.unwrap(),
				);

				let details =
					self.eth.get_bridge_transfer_details_counterparty(transfer_id).await?;
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
