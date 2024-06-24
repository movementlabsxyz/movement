use std::collections::HashMap;

use rand::Rng;
use thiserror::Error;

use bridge_shared::types::{
	Amount, BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId,
	GenUniqueHash, HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock,
};

#[derive(Debug)]
pub enum InitiatorCall<A, H> {
	InitiateBridgeTransfer(InitiatorAddress<A>, RecipientAddress<A>, Amount, TimeLock, HashLock<H>),
	CompleteBridgeTransfer(BridgeTransferId<H>, HashLockPreImage),
}

#[derive(Debug)]
pub struct SmartContractInitiator<A, H, R> {
	pub initiated_transfers: HashMap<BridgeTransferId<H>, BridgeTransferDetails<A, H>>,
	pub rng: R,
}

#[derive(Error, Debug)]
pub enum SmartContractInitiatorError {
	#[error("Failed to initiate bridge transfer")]
	InitiateTransferError,
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

impl<A, H, R> SmartContractInitiator<A, H, R>
where
	A: BridgeAddressType,
	H: BridgeHashType + GenUniqueHash,
	R: Rng,
{
	pub fn new(rng: R) -> Self {
		Self { initiated_transfers: HashMap::new(), rng }
	}

	pub fn initiate_bridge_transfer(
		&mut self,
		initiator: InitiatorAddress<A>,
		recipient: RecipientAddress<A>,
		amount: Amount,
		time_lock: TimeLock,
		hash_lock: HashLock<H>,
	) {
		let bridge_tranfer_id = BridgeTransferId::<H>::gen_unique_hash(&mut self.rng);
		// initiate bridge transfer
		self.initiated_transfers.insert(
			bridge_tranfer_id.clone(),
			BridgeTransferDetails {
				bridge_transfer_id: bridge_tranfer_id,
				initiator_address: initiator,
				recipient_address: recipient,
				hash_lock,
				time_lock,
				amount,
			},
		);
	}

	pub fn complete_bridge_transfer(
		&mut self,
		accounts: &mut HashMap<A, Amount>,
		transfer_id: BridgeTransferId<H>,
		_secret: HashLockPreImage,
	) -> Result<(), SmartContractInitiatorError> {
		// complete bridge transfer
		let transfer = self
			.initiated_transfers
			.get(&transfer_id)
			.ok_or(SmartContractInitiatorError::TransferNotFound)?;

		// let hash = calculate_hash(&secret.0);
		//
		// if transfer.hash_lock != hash {
		// 	return Err(SmartContractInitiatorError::InvalidHashLockPreImage);
		// }

		let balance = accounts.entry((*transfer.recipient_address).clone()).or_insert(Amount(0));
		**balance += *transfer.amount;

		Ok(())
	}
}
