use std::collections::HashMap;
use std::ops::{AddAssign};
use crate::types::{
	Amount, BridgeAddressType, BridgeHashType, BridgeTransferId, CounterpartyCompletedDetails, GenUniqueHash, HashLock, HashLockPreImage, InitiatorAddress, LockDetails, RecipientAddress, TimeLock
};
use std::fmt::Debug;
use thiserror::Error;

pub type SCCResult<A, H, V> =
	Result<SmartContractCounterpartyEvent<A, H, V>, SmartContractCounterpartyError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartContractCounterpartyEvent<A, H, V> {
	LockedBridgeTransfer(LockDetails<A, H, V>),
	CompletedBridgeTransfer(CounterpartyCompletedDetails<A, H, V>),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SmartContractCounterpartyError {
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug)]
pub enum CounterpartyCall<A, H, V> {
	CompleteBridgeTransfer(BridgeTransferId<H>, HashLockPreImage),
	LockBridgeTransfer(
		BridgeTransferId<H>,
		HashLock<H>,
		TimeLock,
		InitiatorAddress<Vec<u8>>,
		RecipientAddress<A>,
		Amount<V>,
	),
}

#[derive(Debug)]
pub struct SmartContractCounterparty<A, H, V> {
	pub locked_transfers: HashMap<BridgeTransferId<H>, LockDetails<A, H, V>>,
}

impl<A, H, V> SmartContractCounterparty<A, H, V>
where
	A: BridgeAddressType + From<RecipientAddress<A>>,
	H: BridgeHashType + GenUniqueHash,
	H: From<HashLockPreImage>,
	V: Copy + Clone + Debug + AddAssign<V>,
{
	pub fn new() -> Self {
		Self { locked_transfers: HashMap::new() }
	}

	pub fn lock_bridge_transfer(
		&mut self,

		bridge_transfer_id: BridgeTransferId<H>,
		hash_lock: HashLock<H>,
		time_lock: TimeLock,
		initiator_address: InitiatorAddress<Vec<u8>>,
		recipient_address: RecipientAddress<A>,
		amount: Amount<V>,
	) -> SCCResult<A, H, V> {
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
		accounts: &mut HashMap<A, Amount<V>>,
		bridge_transfer_id: &BridgeTransferId<H>,
		pre_image: HashLockPreImage,
	) -> SCCResult<A, H, V> {
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
		// let balance = accounts.entry(account).or_insert(Amount(0));
		
		// match transfer.amount.0 {
		// 	_ => balance += transfer.amount.0,
		// 	// BridgedToken::Eth(v) => **balance += v,
		// 	// BridgedToken::Weth(v) => **balance += v,
		// 	// BridgedToken::WethAndEth((w, v)) => {
		// 	// 	todo!();
		// 	// 	// **balance += weth_amount;
		// 	// 	// **balance += eth_amount;
		// 	// }
		// }

		Ok(SmartContractCounterpartyEvent::CompletedBridgeTransfer(
			CounterpartyCompletedDetails::from_lock_details(transfer, pre_image),
		))
	}
}
