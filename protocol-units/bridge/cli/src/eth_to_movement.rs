#![allow(unused_imports)]
use crate::clap::eth_to_movement::EthSharedArgs;
use alloy::primitives::keccak256;
use anyhow::Result;
use bridge_service::{
	chains::{
		bridge_contracts::BridgeContract,
		ethereum::{
			client::{Config, EthClient},
			types::EthAddress,
		},
		movement::{client::MovementClient, utils::MovementAddress},
	},
	types::{
		Amount, AssetType, BridgeAddress, BridgeTransferDetails, BridgeTransferId, HashLock,
		HashLockPreImage,
	},
};
use clap::{Args, Subcommand};

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
