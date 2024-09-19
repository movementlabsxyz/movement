use std::collections::HashMap;
use thiserror::Error;

use rand::Rng;

use crate::{
	bridge_monitoring::BridgeContractInitiatorEvent,
	types::{
		Amount, BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId,
		GenUniqueHash, HashLock, HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock,
	},
};

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

#[derive(Debug)]
pub struct SmartContractInitiator<A, H, R> {
	pub initiated_transfers: HashMap<BridgeTransferId<H>, BridgeTransferDetails<A, H>>,
	pub accounts: HashMap<A, Amount>,
	pub rng: R,
}

impl<A, H, R> SmartContractInitiator<A, H, R>
where
	A: BridgeAddressType,
	H: BridgeHashType + GenUniqueHash + From<HashLockPreImage>,
	R: Rng,
{
	pub fn new(rng: R) -> Self {
		Self { initiated_transfers: HashMap::new(), accounts: HashMap::default(), rng }
	}

	pub fn initiate_bridge_transfer(
		&mut self,
		initiator: InitiatorAddress<A>,
		recipient: RecipientAddress<Vec<u8>>,
		amount: Amount,
		hash_lock: HashLock<H>,
	) -> SCIResult<A, H> {
		let bridge_transfer_id = BridgeTransferId::<H>::gen_unique_hash(&mut self.rng);

		tracing::trace!(
			"SmartContractInitiator: Initiating bridge transfer: {:?}",
			bridge_transfer_id
		);

		// // TODO: fix this
		// let balance = self.accounts.entry(initiator.0.clone()).or_insert(Amount(0));
		// **balance -= amount.0;

		// initiate bridge transfer
		self.initiated_transfers.insert(
			bridge_transfer_id.clone(),
			BridgeTransferDetails {
				bridge_transfer_id: bridge_transfer_id.clone(),
				initiator_address: initiator.clone(),
				recipient_address: recipient.clone(),
				hash_lock: hash_lock.clone(),
				amount,
				state: 1,
			},
		);

		Ok(SmartContractInitiatorEvent::InitiatedBridgeTransfer(BridgeTransferDetails {
			bridge_transfer_id,
			initiator_address: initiator,
			recipient_address: recipient,
			hash_lock,
			amount,
			state: 1,
		}))
	}

	pub fn complete_bridge_transfer(
		&mut self,
		_accounts: &mut HashMap<A, Amount>,
		transfer_id: BridgeTransferId<H>,
		pre_image: HashLockPreImage,
	) -> SCIResult<A, H> {
		tracing::trace!("SmartContractInitiator: Completing bridge transfer: {:?}", transfer_id);

		// complete bridge transfer
		let transfer = self
			.initiated_transfers
			.get(&transfer_id)
			.ok_or(SmartContractInitiatorError::TransferNotFound)?;

		// check if the secret is correct
		let secret_hash = H::from(pre_image.clone());
		if transfer.hash_lock.0 != secret_hash {
			tracing::warn!(
				"Invalid hash lock pre image {pre_image:?} hash {secret_hash:?} != hash_lock {:?}",
				transfer.hash_lock.0
			);
			return Err(SmartContractInitiatorError::InvalidHashLockPreImage);
		}

		Ok(SmartContractInitiatorEvent::CompletedBridgeTransfer(transfer_id))
	}
}
