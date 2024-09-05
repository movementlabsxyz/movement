use alloy::signers::local::PrivateKeySigner;
use clap::{Args, Subcommand};
use ethereum_bridge::types::EthAddress;
use movement_bridge::utils::MovementAddress;

use url::Url;

#[derive(Args, Clone, Debug)]
pub struct EthSharedArgs {
	/// Private key of the Ethereum signer
	#[arg(long)]
	pub eth_private_key: PrivateKeySigner,

	/// URL for the Ethereum RPC
	#[arg(long, default_value = "http://localhost:8545")]
	pub eth_rpc_url: Url,

	/// URL for the Ethereum WebSocket
	#[arg(long, default_value = "ws://localhost:8545")]
	pub eth_ws_url: Url,

	/// Ethereum contract address for the initiator
	#[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
	pub eth_initiator_contract: EthAddress,

	/// Ethereum contract address for the counterparty
	#[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
	pub eth_counterparty_contract: EthAddress,

	/// Ethereum contract address for the counterparty
	#[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
	pub eth_weth_contract: EthAddress,

	/// Gas limit for Ethereum transactions
	#[arg(long, default_value_t = 10_000_000_000)]
	pub eth_gas_limit: u64,
}

#[derive(Args, Clone, Debug)]
pub struct MoveSharedArgs {
	#[arg(long)]
	pub move_private_key: String,

	/// URL for the Ethereum RPC
	#[arg(long, default_value = "http://localhost:8545")]
	pub move_rpc_url: String,

	/// URL for the Ethereum WebSocket
	#[arg(long, default_value = "ws://localhost:8545")]
	pub move_ws_url: String,

	#[arg(long, default_value = "4")]
	pub move_chain_id: String,

	/// Ethereum contract address for the initiator
	#[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
	pub move_initiator_contract: MovementAddress,

	/// Ethereum contract address for the counterparty
	#[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
	pub move_counterparty_contract: MovementAddress,

	/// Gas limit for Ethereum transactions
	#[arg(long, default_value_t = 10_000_000_000)]
	pub move_gas_limit: u64,
}

#[derive(Args, Clone, Debug)]
pub struct CombinedArgs {
    #[command(flatten)]
    pub eth_args: EthSharedArgs,
    
    #[command(flatten)]
    pub move_args: MoveSharedArgs,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Initiate a bridge transfer from Ethereum
	FromEthereum {
		#[command(flatten)]
		args: EthSharedArgs,
		/// The recipient address on the movement labs chain
		recipient: MovementAddress,
		/// The amount of Ethereum to transfer in WEI
		amount: u64,
	},
	/// Initiate a bridge transfer from Movement
	FromMovement {
		#[command(flatten)]
		args: MoveSharedArgs,
		/// The recipient address on the Ethereum chain
		recipient: EthAddress,
		/// The amount of Ethereum to transfer in WEI
		amount: u64,
	},
	/// Resume a bridge transfer
	LockOnEthereum {
		#[command(flatten)]
		args: EthSharedArgs,

		#[arg(long)]
		amount: u64,

		/// The address of the originator on ethereum
		#[arg(long)]
		originator: MovementAddress,

		/// The address of the recipient on the movement
		#[arg(long)]
		recipient: EthAddress,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Resume a bridge transfer
	LockOnMovement {
		#[command(flatten)]
		args: MoveSharedArgs,

		#[arg(long)]
		amount: u64,

		/// The address of the originator on ethereum
		#[arg(long)]
		originator: EthAddress,

		/// The address of the recipient on the movement
		#[arg(long)]
		recipient: MovementAddress,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Complete a bridge transfer
	CompleteInitiatorOnEthereum {
		#[command(flatten)]
		args: EthSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Complete a bridge transfer
	CompleteInitiatorOnMovement {
		#[command(flatten)]
		args: MoveSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Complete a bridge transfer
	CompleteCounterpartyOnEthereum {
		#[command(flatten)]
		args: EthSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Complete a bridge transfer
	CompleteCounterpartyOnMovement {
		#[command(flatten)]
		args: MoveSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Cancel a bridge transfer
	CancelOnEthereum {
		#[command(flatten)]
		args: EthSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Cancel a bridge transfer
	CancelOnMovement {
		#[command(flatten)]
		args: MoveSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Cancel a bridge transfer
	RefundOnEthereum {
		#[command(flatten)]
		args: EthSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	/// Cancel a bridge transfer
	RefundOnMovement {
		#[command(flatten)]
		args: MoveSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
	IniatializeUser {
		#[command(flatten)]
		args: CombinedArgs,
	},
}
