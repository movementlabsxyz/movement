use std::collections::HashMap;

use bridge_shared::types::{
	Amount, BridgeAddressType, BridgeHashType, BridgeTransferId, CompletedDetails, GenUniqueHash,
	HashLock, HashLockPreImage, LockDetails, RecipientAddress, TimeLock,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartContractCounterpartyEvent<H> {
	LockedBridgeTransfer(LockDetails<H>),
	CompletedBridgeTransfer(CompletedDetails<H>),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SmartContractCounterpartyError {
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug)]
pub enum CounterpartyCall<H> {
	CompleteBridgeTransfer(BridgeTransferId<H>, HashLockPreImage),
	LockBridgeTransfer(BridgeTransferId<H>, HashLock<H>, TimeLock, RecipientAddress, Amount),
}

#[derive(Debug)]
pub struct SmartContractCounterparty<A, H> {
	pub locked_transfers: HashMap<BridgeTransferId<H>, LockDetails<H>>,
	pub _phantom: std::marker::PhantomData<A>,
}

pub type SCCResult<H> = Result<SmartContractCounterpartyEvent<H>, SmartContractCounterpartyError>;

impl<A, H> SmartContractCounterparty<A, H>
where
	A: BridgeAddressType + From<RecipientAddress>,
	H: BridgeHashType + GenUniqueHash,
	H: From<HashLockPreImage>,
{
	pub fn new() -> Self {
		Self { locked_transfers: HashMap::new(), _phantom: std::marker::PhantomData }
	}

	pub fn lock_bridge_transfer(
		&mut self,

		bridge_transfer_id: BridgeTransferId<H>,
		hash_lock: HashLock<H>,
		time_lock: TimeLock,
		recipient_address: RecipientAddress,
		amount: Amount,
	) -> SCCResult<H> {
		tracing::trace!(
			"SmartContractCounterparty: Locking bridge transfer: {:?}",
			bridge_transfer_id
		);
		self.locked_transfers.insert(
			bridge_transfer_id.clone(),
			LockDetails {
				bridge_transfer_id: bridge_transfer_id.clone(),
				recipient_address: recipient_address.clone(),
				hash_lock: hash_lock.clone(),
				time_lock: time_lock.clone(),
				amount,
			},
		);

		Ok(SmartContractCounterpartyEvent::LockedBridgeTransfer(LockDetails {
			bridge_transfer_id,
			recipient_address,
			hash_lock,
			time_lock,
			amount,
		}))
	}

	pub fn complete_bridge_transfer(
		&mut self,
		accounts: &mut HashMap<A, Amount>,
		bridge_transfer_id: &BridgeTransferId<H>,
		pre_image: HashLockPreImage,
	) -> SCCResult<H> {
		let transfer = self
			.locked_transfers
			.remove(bridge_transfer_id)
			.ok_or(SmartContractCounterpartyError::TransferNotFound)?;

		tracing::trace!("SmartContractCounterparty: Completing bridge transfer: {:?}", transfer);

		// check if the secret is correct
		let secret_hash = H::from(pre_image.clone());
		if transfer.hash_lock.0 != secret_hash {
			tracing::warn!(
				"Invalid hash lock pre image {pre_image:?} hash {secret_hash:?} != hash_lock {:?}",
				transfer.hash_lock.0
			);
			return Err(SmartContractCounterpartyError::InvalidHashLockPreImage);
		}

		// TODO: fix this
		let account = A::from(transfer.recipient_address.clone());
		let balance = accounts.entry(account).or_insert(Amount(0));
		**balance += *transfer.amount;

		Ok(SmartContractCounterpartyEvent::CompletedBridgeTransfer(
			CompletedDetails::from_lock_details(transfer, pre_image),
		))
	}
}
