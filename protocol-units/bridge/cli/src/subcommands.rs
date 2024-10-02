use bridge_service::{
	chains::{ethereum::types::EthAddress, movement::utils::MovementAddress},
	types::BridgeAddress,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Movementlabs Bridge CLI")]
#[command(about = "Command line interface to perform atomic bridge transfers", long_about = None)]
pub struct CliOptions {
	/// The bridge command to run (Ethereum to Movement or Movement to Ethereum)
	#[command(subcommand)]
	pub command: BridgeCommands,
}

#[derive(Subcommand)]
pub enum BridgeCommands {
	/// Ethereum to Movement bridge commands
	#[command(subcommand)]
	L1toL2(EthSubCommands),

	/// Movement to Ethereum bridge commands
	#[command(subcommand)]
	L2toL1(MovementSubCommands),
}

#[derive(Subcommand)]
pub enum EthSubCommands {
	/// Initiate a bridge transfer from Ethereum to Movement
	#[command(about = "Initiates an Ethereum to Movement bridge transfer")]
	Initiate {
		/// The recipient's Movement address
		#[arg(help = "Recipient's Movement address (as hex)")]
		recipient: BridgeAddress<Vec<u8>>,

		/// The amount of ETH to transfer
		#[arg(help = "Amount of ETH to transfer")]
		amount: u64,

		/// The hash lock for the bridge transfer
		#[arg(help = "Hash lock for the transfer (as hex)")]
		hash_lock: String,
	},

	/// Complete an Ethereum bridge transfer
	#[command(about = "Completes an Ethereum to Movement bridge transfer")]
	Complete {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,

		/// The pre-image for the hash lock
		#[arg(help = "Pre-image (as hex)")]
		pre_image: String,
	},

	/// Refund an Ethereum bridge transfer
	#[command(about = "Refunds an Ethereum bridge transfer")]
	Refund {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,
	},

	/// Lock an Ethereum bridge transfer
	#[command(about = "Locks an Ethereum bridge transfer")]
	Lock {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,

		/// The initiator address
		#[arg(help = "Initiator address (as hex)")]
		initiator: String,

		/// The recipient's Ethereum address
		#[arg(help = "Recipient's Ethereum address")]
		recipient: BridgeAddress<EthAddress>,

		/// The amount of ETH to lock
		#[arg(help = "Amount of ETH to lock")]
		amount: u64,

		/// The hash lock for the bridge transfer
		#[arg(help = "Hash lock for the transfer (as hex)")]
		hash_lock: String,
	},

	/// Abort an Ethereum bridge transfer
	#[command(about = "Aborts an Ethereum bridge transfer")]
	Abort {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,
	},

	/// Get details of an Ethereum initiator bridge transfer
	#[command(about = "Gets the details of an Ethereum initiator bridge transfer")]
	DetailsInitiator {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,
	},

	/// Get details of an Ethereum counterparty bridge transfer
	#[command(about = "Gets the details of an Ethereum counterparty bridge transfer")]
	DetailsCounterparty {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,
	},
}

#[derive(Subcommand)]
pub enum MovementSubCommands {
	/// Initiate a bridge transfer from Movement to Ethereum
	#[command(about = "Initiates a Movement to Ethereum bridge transfer")]
	Initiate {
		/// The recipient's Ethereum address
		#[arg(help = "Recipient's Ethereum address (as hex)")]
		recipient: BridgeAddress<Vec<u8>>,

		/// The amount of MOVETH to transfer
		#[arg(help = "Amount of MOVETH to transfer")]
		amount: u64,

		/// The hash lock for the bridge transfer
		#[arg(help = "Hash lock for the transfer (as hex)")]
		hash_lock: String,
	},

	/// Complete a Movement bridge transfer
	#[command(about = "Completes a Movement to Ethereum bridge transfer")]
	Complete {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,

		/// The pre-image for the hash lock
		#[arg(help = "Pre-image (as hex)")]
		pre_image: String,
	},

	/// Refund a Movement bridge transfer
	#[command(about = "Refunds a Movement bridge transfer")]
	Refund {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,
	},

	/// Lock a Movement bridge transfer
	#[command(about = "Locks a Movement bridge transfer")]
	Lock {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,

		/// The initiator address
		#[arg(help = "Initiator address (as hex)")]
		initiator: String,

		/// The recipient's Movement address
		#[arg(help = "Recipient's Movement address")]
		recipient: BridgeAddress<MovementAddress>,

		/// The amount of MOVETH to lock
		#[arg(help = "Amount of MOVETH to lock")]
		amount: u64,

		/// The hash lock for the bridge transfer
		#[arg(help = "Hash lock for the transfer (as hex)")]
		hash_lock: String,
	},

	/// Abort a Movement bridge transfer
	#[command(about = "Aborts a Movement bridge transfer")]
	Abort {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,
	},

	/// Get details of a Movement initiator bridge transfer
	#[command(about = "Gets the details of a Movement initiator bridge transfer")]
	DetailsInitiator {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,
	},

	/// Get details of a Movement counterparty bridge transfer
	#[command(about = "Gets the details of a Movement counterparty bridge transfer")]
	DetailsCounterparty {
		/// The transfer ID of the bridge transfer
		#[arg(help = "Transfer ID (as hex)")]
		transfer_id: String,
	},
}
