use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Movement Atomic Bridge CLI")]
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
	EthToMovement(TransferSubcommands),

	/// Movement to Ethereum bridge commands
	#[command(subcommand)]
	MovementToEth(TransferSubcommands),
}

/// The Transfer C
#[derive(Subcommand)]
pub enum TransferSubcommands {
	/// Initiate a bridge transfer from Ethereum to Movement
	#[command(about = "Initiates a transfer from the origin chain to the destination chain")]
	Initiate {
		/// The recipient's Movement address
		#[arg(help = "The recipient address on the destination chain")]
		recipient: String,

		/// The amount of ETH to transfer
		#[arg(help = "Amount of MOVE to transfer")]
		amount: u64,

		/// The hash lock for the bridge transfer
		#[arg(help = "keccak256 hash of the secret")]
		hash_lock: String,
	},

	#[command(
		about = "Completes the transfer by calling complete_bridge_transfer on the destination chain and revealing the preimage"
	)]
	Complete {
		/// The transfer id of the bridge transfer
		#[arg(help = "The transfer id of the bridge transfer")]
		transfer_id: String,

		/// The premiage for the bridge transfer
		#[arg(help = "The preimage, or secret")]
		preimage: String,
	},

	/// Refund an Ethereum bridge transfer
	#[command(
		about = "Refunds a transfer back to the origin chain. Only callable by the Owner of the contract"
	)]
	InitiatorRefund {
		/// The transfer ID of the bridge transfer
		#[arg(help = "The transfer id of the bridge transfer")]
		transfer_id: String,
	},

	CounterpartyAbort {
		/// The Transfer ID of the bridge transfer
		#[arg(help = "The transfer id of the bridge transfer")]
		transfer_id: String,
	},

	/// Get details of an Ethereum initiator bridge transfer
	#[command(
		about = "Gets the details of an Ethereum to Movement transfer from the Ethereum Initiator contract"
	)]
	Details {
		/// The transfer ID of the bridge transfer
		#[arg(help = "The transfer id")]
		transfer_id: String,
	},
}
