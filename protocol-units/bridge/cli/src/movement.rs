use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "movement-cli")]
#[command(about = "CLI for interacting with the Movement client", long_about = None)]
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

pub async fn run(command: &MovementCommands) -> Result<()> {
	match command {
		MovementCommands::Deploy { config_path } => {
			//load_config()
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
			//load_config()
			todo!()
		}
		MovementCommands::Complete { config_path, bridge_transfer_id, pre_image } => {
			//load_config()
			todo!()
		}
		MovementCommands::Abort { config_path, bridge_transfer_id } => {
			//load_config()
			todo!()
		}
		MovementCommands::Details { config_path, bridge_transfer_id } => {
			//load_config()
			todo!()
		}
	}

	Ok(())
}
