use alloy_primitives::{Address as EthAddress, FixedBytes, U256};
use anyhow::Result;
use bridge_shared::bridge_contracts::BridgeContractInitiator;
use clap::{Parser, Subcommand};
use ethereum_bridge::{Config, EthClient};

#[derive(Parser)]
#[command(name = "eth-bridge-cli")]
#[command(about = "CLI for interacting with the Ethereum bridge client", long_about = None)]
struct EthCli {
	#[command(subcommand)]
	command: EthCommands,
}

#[derive(Subcommand)]
pub enum EthCommands {
	Deploy {
		#[arg(short, long)]
		config_path: String,
	},
	Initiate {
		#[arg(short, long)]
		config_path: String,
		#[arg(short, long)]
		initiator_address: String,
		#[arg(short, long)]
		recipient_address: String,
		#[arg(short, long)]
		hash_lock: String,
		#[arg(short, long)]
		time_lock: u64,
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
	Refund {
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

pub async fn run(command: &EthCommands) -> Result<()> {
	match command {
		EthCommands::Deploy { config_path } => {
			let config = load_config(config_path)?;
			deploy(config).await?;
		}
		EthCommands::Initiate {
			config_path,
			initiator_address,
			recipient_address,
			hash_lock,
			time_lock,
			amount,
		} => {
			let config = load_config(config_path)?;
			initiate_transfer(
				config,
				initiator_address,
				recipient_address,
				hash_lock,
				*time_lock,
				*amount,
			)
			.await?;
		}
		EthCommands::Complete { config_path, bridge_transfer_id, pre_image } => {
			let config = load_config(config_path)?;
			complete_transfer(config, bridge_transfer_id, pre_image).await?;
		}
		EthCommands::Refund { config_path, bridge_transfer_id } => {
			let config = load_config(config_path)?;
			refund_transfer(config, bridge_transfer_id).await?;
		}
		EthCommands::Details { config_path, bridge_transfer_id } => {
			let config = load_config(config_path)?;
			get_transfer_details(config, bridge_transfer_id).await?;
		}
	}

	Ok(())
}

fn load_config(path: &str) -> Result<Config> {
	// Load the config from the provided path
	let config_str = std::fs::read_to_string(path)?;
	let config: Config = serde_json::from_str(&config_str)?;
	Ok(config)
}

async fn deploy(config: Config) -> Result<()> {
	// Implement the deploy logic here
	println!("Deploying with config: {:?}", config);
	Ok(())
}

async fn initiate_transfer(
	config: Config,
	initiator_address: &str,
	recipient_address: &str,
	hash_lock: &str,
	time_lock: u64,
	amount: u64,
) -> Result<()> {
	let mut client = EthClient::build_with_config(config, recipient_address).await?;
	let chain_id = 42; //dummy value for now
	client
		.initiate_bridge_transfer(
			EthAddress::parse_checksummed(initiator_address, Some(chain_id))?,
			recipient_address,
			hash_lock,
			time_lock,
			amounts,
		)
		.await?;
	Ok(())
}

async fn complete_transfer(
	config: Config,
	bridge_transfer_id: &str,
	pre_image: &str,
) -> Result<()> {
	let mut client = EthClient::build_with_config(config, "").await?;
	client.complete_bridge_transfer(bridge_transfer_id, pre_image).await?;
	Ok(())
}

async fn refund_transfer(config: Config, bridge_transfer_id: &str) -> Result<()> {
	let mut client = EthClient::build_with_config(config, "").await?;
	client.refund_bridge_transfer(bridge_transfer_id).await?;
	Ok(())
}

async fn get_transfer_details(config: Config, bridge_transfer_id: &str) -> Result<()> {
	let mut client = EthClient::build_with_config(config, "").await?;
	let details = client.get_bridge_transfer_details(bridge_transfer_id).await?;
	println!("Transfer details: {:?}", details);
	Ok(())
}
