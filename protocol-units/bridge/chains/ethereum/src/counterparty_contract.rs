use bridge_shared::bridge_contracts::{
	BridgeContractCounterparty, BridgeContractCounterpartyResult,
};
use bridge_shared::bridge_monitoring::BridgeContractCounterpartyEvent;
use bridge_shared::types::{
	Amount, AssetType, BridgeTransferDetails, BridgeTransferId, CounterpartyCompletedDetails,
	HashLock, HashLockPreImage, InitiatorAddress, LockDetails, RecipientAddress, TimeLock,
};

use std::collections::HashMap;
use std::fmt::Debug;
use thiserror::Error;

use crate::types::{EthAddress, EthHash};

pub type SCCResult<A, H> =
	Result<SmartContractCounterpartyEvent<A, H>, SmartContractCounterpartyError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartContractCounterpartyEvent<A, H> {
	LockedBridgeTransfer(LockDetails<A, H>),
	CompletedBridgeTransfer(CounterpartyCompletedDetails<A, H>),
}

impl<A, H> From<BridgeContractCounterpartyEvent<A, H>> for SmartContractCounterpartyEvent<A, H> {
	fn from(event: BridgeContractCounterpartyEvent<A, H>) -> Self {
		match event {
			BridgeContractCounterpartyEvent::Locked(details) => {
				SmartContractCounterpartyEvent::LockedBridgeTransfer(details)
			}
			BridgeContractCounterpartyEvent::Completed(details) => {
				SmartContractCounterpartyEvent::CompletedBridgeTransfer(details)
			}
		}
	}
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SmartContractCounterpartyError {
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug)]
pub enum CounterpartyCall<A, H> {
	CompleteBridgeTransfer(BridgeTransferId<H>, HashLockPreImage),
	LockBridgeTransfer(
		BridgeTransferId<H>,
		HashLock<H>,
		TimeLock,
		InitiatorAddress<Vec<u8>>,
		RecipientAddress<A>,
		Amount,
	),
}

#[derive(Debug, Clone)]
pub struct EthSmartContractCounterparty {
	pub locked_transfers: HashMap<BridgeTransferId<EthHash>, LockDetails<EthAddress, EthHash>>,
}

impl EthSmartContractCounterparty {
	pub fn new() -> Self {
		Self { locked_transfers: HashMap::new() }
	}

	pub fn lock_bridge_transfer(
		&mut self,

		bridge_transfer_id: BridgeTransferId<EthHash>,
		hash_lock: HashLock<EthHash>,
		time_lock: TimeLock,
		initiator_address: InitiatorAddress<Vec<u8>>,
		recipient_address: RecipientAddress<EthAddress>,
		amount: Amount,
	) -> SCCResult<EthAddress, EthHash> {
		tracing::trace!(
			"SmartContractCounterparty: Locking bridge transfer: {:?}",
			bridge_transfer_id
		);
		self.locked_transfers.insert(
			bridge_transfer_id.clone(),
			LockDetails {
				bridge_transfer_id: bridge_transfer_id.clone(),
				initiator_address: initiator_address.clone(),
				recipient_address: recipient_address.clone(),
				hash_lock: hash_lock.clone(),
				time_lock: time_lock.clone(),
				amount,
			},
		);

		Ok(SmartContractCounterpartyEvent::LockedBridgeTransfer(LockDetails {
			bridge_transfer_id,
			initiator_address,
			recipient_address,
			hash_lock,
			time_lock,
			amount,
		}))
	}

	pub fn complete_bridge_transfer(
		&mut self,
		accounts: &mut HashMap<EthAddress, Amount>,
		bridge_transfer_id: &BridgeTransferId<EthHash>,
		pre_image: HashLockPreImage,
	) -> SCCResult<EthAddress, EthHash> {
		let transfer = self
			.locked_transfers
			.remove(bridge_transfer_id)
			.ok_or(SmartContractCounterpartyError::TransferNotFound)?;

		tracing::trace!("SmartContractCounterparty: Completing bridge transfer: {:?}", transfer);

		// check if the secret is correct
		let secret_hash = EthHash::from(pre_image.clone());
		if transfer.hash_lock.0 != secret_hash {
			tracing::warn!(
				"Invalid hash lock pre image {pre_image:?} hash {secret_hash:?} != hash_lock {:?}",
				transfer.hash_lock.0
			);
			return Err(SmartContractCounterpartyError::InvalidHashLockPreImage);
		}

		// TODO: fix this
		let account = EthAddress::from(transfer.recipient_address.clone());

		let balance = accounts.entry(account).or_insert(Amount(AssetType::EthAndWeth((0, 0))));
		// balance += **transfer.amount;

		Ok(SmartContractCounterpartyEvent::CompletedBridgeTransfer(
			CounterpartyCompletedDetails::from_lock_details(transfer, pre_image),
		))
	}
}

#[async_trait::async_trait]
impl BridgeContractCounterparty for EthSmartContractCounterparty {
	type Address = EthAddress;
	type Hash = EthHash;

	async fn lock_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_hash_lock: HashLock<Self::Hash>,
		_time_lock: TimeLock,
		_initiator: InitiatorAddress<Vec<u8>>,
		_recipient: RecipientAddress<Self::Address>,
		_amount: Amount,
	) -> BridgeContractCounterpartyResult<()> {
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()> {
		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>>
	{
		Ok(None)
	}
}
