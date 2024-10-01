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

#[derive(Subcommand)]
pub enum Commands {
	/// Initiate a bridge transfer
	Initiate {
		#[command(flatten)]
		args: EthSharedArgs,

		/// The recipient address on the movement labs chain
		recipient: MovementAddress,

		/// The amount of Ethereum to transfer in WEI
		amount: u64,
	},
	/// Resume a bridge transfer
	Complete {
		#[command(flatten)]
		args: EthSharedArgs,

		/// The ID of the transfer to resume
		#[arg(long)]
		transfer_id: String,
	},
}
