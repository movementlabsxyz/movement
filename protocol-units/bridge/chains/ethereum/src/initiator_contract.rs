use crate::types::{EthAddress, EthHash};
use bridge_shared::{
	bridge_contracts::{BridgeContractInitiator, BridgeContractInitiatorResult},
	bridge_monitoring::BridgeContractInitiatorEvent,
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
		InitiatorAddress, RecipientAddress, TimeLock,
	},
};
use std::sync::Arc;
use std::{collections::HashMap, sync::RwLock};
use thiserror::Error;

pub type SCIResult<A, H> = Result<SmartContractInitiatorEvent<A, H>, SmartContractInitiatorError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartContractInitiatorEvent<A, H> {
	InitiatedBridgeTransfer(BridgeTransferDetails<A, H>),
	CompletedBridgeTransfer(BridgeTransferId<H>),
	RefundedBridgeTransfer(BridgeTransferId<H>),
}

impl<A, H> From<BridgeContractInitiatorEvent<A, H>> for SmartContractInitiatorEvent<A, H> {
	fn from(event: BridgeContractInitiatorEvent<A, H>) -> Self {
		match event {
			BridgeContractInitiatorEvent::Initiated(details) => {
				SmartContractInitiatorEvent::InitiatedBridgeTransfer(details)
			}
			BridgeContractInitiatorEvent::Completed(id) => {
				SmartContractInitiatorEvent::CompletedBridgeTransfer(id)
			}
			BridgeContractInitiatorEvent::Refunded(id) => {
				SmartContractInitiatorEvent::RefundedBridgeTransfer(id)
			}
		}
	}
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SmartContractInitiatorError {
	#[error("Failed to initiate bridge transfer")]
	InitiateTransferError,
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitiatorEvent<A, H> {
	Initiated(BridgeTransferDetails<A, H>),
	Completed(BridgeTransferId<H>),
	Refunded(BridgeTransferId<H>),
}

#[derive(Debug)]
pub enum InitiatorCall<A, H> {
	InitiateBridgeTransfer(
		InitiatorAddress<A>,
		RecipientAddress<Vec<u8>>,
		Amount,
		TimeLock,
		HashLock<H>,
	),
	CompleteBridgeTransfer(BridgeTransferId<H>, HashLockPreImage),
}

#[derive(Debug, Clone)]
pub struct EthSmartContractInitiator {
	pub initiated_transfers:
		Arc<RwLock<HashMap<BridgeTransferId<EthHash>, BridgeTransferDetails<EthAddress, EthHash>>>>,
	pub accounts: HashMap<EthAddress, Amount>,
}

impl EthSmartContractInitiator {
	pub fn new() -> Self {
		Self {
			initiated_transfers: Arc::new(RwLock::new(HashMap::new())),
			accounts: HashMap::default(),
		}
	}

	pub fn initiate_bridge_transfer(
		&mut self,
		initiator: InitiatorAddress<EthAddress>,
		recipient: RecipientAddress<Vec<u8>>,
		amount: Amount,
		time_lock: TimeLock,
		hash_lock: HashLock<EthHash>,
	) -> SCIResult<EthAddress, EthHash> {
		// Update balance (you might need to adjust this logic based on your requirements)
		// let balance = self.accounts.entry(initiator.0.clone()).or_insert(Amount(0));
		// *balance -= amount.weth();

		// Lock the RwLock and get a mutable reference to the `initiated_transfers`
		let mut initiated_transfers = self.initiated_transfers.write().unwrap();
		let dummy_id = BridgeTransferId(EthHash::random());

		tracing::trace!("SmartContractInitiator: Initiating bridge transfer: {:?}", dummy_id);
		// Insert the new transfer into the map
		initiated_transfers.insert(
			dummy_id.clone(),
			BridgeTransferDetails {
				bridge_transfer_id: dummy_id.clone(),
				initiator_address: initiator.clone(),
				recipient_address: recipient.clone(),
				hash_lock: hash_lock.clone(),
				time_lock: time_lock.clone(),
				amount,
				state: 1,
			},
		);

		Ok(SmartContractInitiatorEvent::InitiatedBridgeTransfer(BridgeTransferDetails {
			bridge_transfer_id: dummy_id,
			initiator_address: initiator,
			recipient_address: recipient,
			hash_lock,
			time_lock,
			amount,
			state: 1,
		}))
	}

	pub fn complete_bridge_transfer(
		&mut self,
		_accounts: &mut HashMap<EthAddress, Amount>,
		transfer_id: BridgeTransferId<EthHash>,
		_pre_image: HashLockPreImage,
	) -> SCIResult<EthAddress, EthHash> {
		tracing::trace!("SmartContractInitiator: Completing bridge transfer: {:?}", transfer_id);

		let initated_transfers = self.initiated_transfers.read().unwrap();

		// complete bridge transfer
		// let transfer = initiated_transfers
		// 	.get(&transfer_id)
		// 	.ok_or(SmartContractInitiatorError::TransferNotFound)?;

		// check if the secret is correct
		// let secret_hash = "You shall not pass!";
		// if transfer.hash_lock.0 != secret_hash {
		// 	tracing::warn!(
		// 		"Invalid hash lock pre image {pre_image:?} hash {secret_hash:?} != hash_lock {:?}",
		// 		transfer.hash_lock.0
		// 	);
		// 	return Err(SmartContractInitiatorError::InvalidHashLockPreImage);
		// }

		Ok(SmartContractInitiatorEvent::CompletedBridgeTransfer(transfer_id))
	}
}

#[async_trait::async_trait]
impl BridgeContractInitiator for EthSmartContractInitiator {
	type Address = EthAddress;
	type Hash = EthHash;

	async fn initiate_bridge_transfer(
		&mut self,
		_initiator_address: InitiatorAddress<Self::Address>,
		_recipient_address: RecipientAddress<Vec<u8>>,
		_hash_lock: HashLock<Self::Hash>,
		_time_lock: TimeLock,
		_amount: Amount,
	) -> BridgeContractInitiatorResult<()> {
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: HashLockPreImage,
	) -> BridgeContractInitiatorResult<()> {
		Ok(())
	}

	async fn refund_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>> {
		Ok(None)
	}
}
