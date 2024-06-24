use std::collections::HashMap;

use bridge_shared::types::{
	Amount, BridgeAddressType, BridgeHashType, BridgeTransferId, GenUniqueHash, HashLock,
	HashLockPreImage, LockDetails, RecipientAddress, TimeLock,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SmartContractCounterpartyError {
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug)]
pub enum CounterpartyCall<A, H> {
	CompleteBridgeTransfer(BridgeTransferId<H>, HashLockPreImage),
	LockBridgeTransfer(BridgeTransferId<H>, HashLock<H>, TimeLock, RecipientAddress<A>, Amount),
}

#[derive(Debug)]
pub struct SmartContractCounterparty<A, H> {
	pub locked_transfers: HashMap<BridgeTransferId<H>, LockDetails<A, H>>,
}

impl<A, H> Default for SmartContractCounterparty<A, H>
where
	A: BridgeAddressType,
	H: BridgeHashType + GenUniqueHash,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<A, H> SmartContractCounterparty<A, H>
where
	A: BridgeAddressType,
	H: BridgeHashType + GenUniqueHash,
{
	pub fn new() -> Self {
		Self { locked_transfers: HashMap::new() }
	}

	pub fn lock_bridge_transfer(
		&mut self,

		bridge_transfer_id: BridgeTransferId<H>,
		hash_lock: HashLock<H>,
		time_lock: TimeLock,
		recipient_address: RecipientAddress<A>,
		amount: Amount,
	) {
		tracing::trace!(
			"SmartContractCounterparty: Locking bridge transfer: {:?}",
			bridge_transfer_id
		);
		self.locked_transfers.insert(
			bridge_transfer_id.clone(),
			LockDetails { bridge_transfer_id, recipient_address, hash_lock, time_lock, amount },
		);
	}

	pub fn complete_bridge_transfer(
		&mut self,
		accounts: &mut HashMap<A, Amount>,
		bridge_transfer_id: &BridgeTransferId<H>,
		pre_image: &HashLockPreImage,
	) -> Result<(), SmartContractCounterpartyError> {
		let transfer = self
			.locked_transfers
			.remove(bridge_transfer_id)
			.ok_or(SmartContractCounterpartyError::TransferNotFound)?;

		tracing::trace!("SmartContractCounterparty: Completing bridge transfer: {:?}", transfer);

		let balance = accounts.entry((*transfer.recipient_address).clone()).or_insert(Amount(0));
		**balance += *transfer.amount;

		Ok(())
	}
}
