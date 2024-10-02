use bridge_service::{
	chains::{ethereum::types::EthAddress, movement::utils::MovementAddress},
	types::BridgeAddress,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Movementlabs Bridge CLI")]
#[command(about = "Command line interface to perform an atomic bridge transfers", long_about = None)]
pub struct CliOptions {
	#[command(subcommand)]
	pub command: BridgeCommands,
}

#[derive(Subcommand)]
pub enum BridgeCommands {
	/// Ethereum to Movement bridge commands
	#[command(subcommand)]
	L1toL2(EthSubCommands),
	/// Movement to Etherum bridge commands
	#[command(subcommand)]
	L2toL1(MovementSubCommands),
}

// These enums are very similar apart from the inner address types
// We could use generics but we get clap shouting about many, may trait bounds missing.
#[derive(Subcommand)]
pub enum EthSubCommands {
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
		recipient: BridgeAddress<EthAddress>,
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

#[derive(Subcommand)]
pub enum MovementSubCommands {
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
		recipient: BridgeAddress<MovementAddress>,
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
