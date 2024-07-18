use anyhow::Result;
use bridge_shared::{
	bridge_contracts::BridgeContractCounterparty,
	types::{
		Amount, BridgeTransferId, HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress,
		TimeLock,
	},
};
use clap::{Parser, Subcommand};
use movement_bridge::{utils::MovementAddress, MovementClient};

#[derive(Parser)]
#[command(name = "movement-cli")]
#[command(about = "Movement Bridge client", long_about = None)]
pub struct MovementCli {
	#[command(subcommand)]
	pub command: MovementCommands,
}

#[derive(Subcommand)]
pub enum MovementCommands {
	Deploy {
		#[arg(short, long)]
		config_path: String,
	},
	LockAssets {
		#[arg(short, long)]
		config_path: String,
		#[arg(short, long)]
		initiator: String,
		#[arg(short, long)]
		bridge_transfer_id: String,
		#[arg(short, long)]
		hash_lock: String,
		#[arg(short, long)]
		time_lock: u64,
		#[arg(short, long)]
		recipient: String,
		#[arg(short, long)]
		amount: u64,
	},
	Complete {
		#[arg(short, long)]
		config_path: String,
		#[arg(short, long)]
		bridge_transfer_id: String,
		#[arg(short, long)]
		pre_image: String,
	},
	Abort {
		#[arg(short, long)]
		config_path: String,
		#[arg(short, long)]
		bridge_transfer_id: String,
	},
	Details {
		#[arg(short, long)]
		config_path: String,
		#[arg(short, long)]
		bridge_transfer_id: String,
	},
}

#[allow(unused)]
pub async fn run(command: &MovementCommands) -> Result<()> {
	match command {
		MovementCommands::Deploy { config_path } => {
			todo!()
		}
		MovementCommands::LockAssets {
			config_path,
			initiator,
			bridge_transfer_id,
			hash_lock,
			time_lock,
			recipient,
			amount,
		} => {
			lock_assets(bridge_transfer_id, hash_lock, *time_lock, recipient, *amount).await?;
		}
		MovementCommands::Complete { config_path, bridge_transfer_id, pre_image } => {
			complete(bridge_transfer_id, pre_image).await?;
		}
		MovementCommands::Abort { config_path, bridge_transfer_id } => {
			abort(bridge_transfer_id).await?;
		}
		MovementCommands::Details { config_path, bridge_transfer_id } => {
			details(bridge_transfer_id).await?;
		}
	}

	Ok(())
}

async fn lock_assets(
	bridge_transfer_id: &str,
	hash_lock: &str,
	time_lock: u64,
	recipient: &str,
	amount: u64,
) -> Result<()> {
	let mut client = MovementClient::build_with_config().await?;
	client
		.lock_bridge_transfer_assets(
			BridgeTransferId::parse(bridge_transfer_id)?,
			HashLock::parse(hash_lock)?,
			TimeLock(time_lock),
			InitiatorAddress(Vec::new()), //dummy for now
			RecipientAddress(MovementAddress::from(recipient)),
			Amount(amount),
		)
		.await?;
	Ok(())
}

async fn complete(bridge_transfer_id: &str, preimage: &str) -> Result<()> {
	let mut client = MovementClient::build_with_config().await?;
	client
		.complete_bridge_transfer(
			BridgeTransferId::parse(bridge_transfer_id)?,
			HashLockPreImage(preimage.into()),
		)
		.await?;
	Ok(())
}

async fn abort(bridge_transfer_id: &str) -> Result<()> {
	let mut client = MovementClient::build_with_config().await?;
	client
		.abort_bridge_transfer(BridgeTransferId::parse(bridge_transfer_id)?)
		.await?;
	Ok(())
}

async fn details(bridge_transfer_id: &str) -> Result<()> {
	let mut client = MovementClient::build_with_config().await?;
	client
		.get_bridge_transfer_details(BridgeTransferId::parse(bridge_transfer_id)?)
		.await?;
	Ok(())
}
