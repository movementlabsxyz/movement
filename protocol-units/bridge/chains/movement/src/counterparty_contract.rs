use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractCounterpartyResult},
	types::{
		Amount, AssetType, BridgeTransferDetails, HashLock, HashLockPreImage, InitiatorAddress,
		RecipientAddress, SCCResult, SmartContractCounterpartyError,
		SmartContractCounterpartyEvent, TimeLock,
	},
};
use std::collections::HashMap;

use bridge_shared::types::{BridgeTransferId, CounterpartyCompletedDetails, LockDetails};
use thiserror::Error;

use crate::utils::{MovementAddress, MovementHash};

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveCounterpartyEvent<A, H> {
	LockedBridgeTransfer(LockDetails<A, H>),
	CompletedBridgeTransfer(CounterpartyCompletedDetails<A, H>),
}

#[allow(unused)]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MoveCounterpartyError {
	#[error("Transfer not found")]
	TransferNotFound,
	#[error("Invalid hash lock pre image (secret)")]
	InvalidHashLockPreImage,
}

#[derive(Debug, Clone)]
pub struct MovementSmartContractCounterparty {
	pub locked_transfers:
		HashMap<BridgeTransferId<MovementHash>, LockDetails<MovementAddress, MovementHash>>,
}

impl MovementSmartContractCounterparty {
	pub fn new() -> Self {
		Self { locked_transfers: HashMap::new() }
	}

	pub fn lock_bridge_transfer(
		&mut self,

		bridge_transfer_id: BridgeTransferId<MovementHash>,
		hash_lock: HashLock<MovementHash>,
		time_lock: TimeLock,
		initiator_address: InitiatorAddress<Vec<u8>>,
		recipient_address: RecipientAddress<MovementAddress>,
		amount: Amount,
	) -> SCCResult<MovementAddress, MovementHash> {
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
		accounts: &mut HashMap<MovementAddress, Amount>,
		bridge_transfer_id: &BridgeTransferId<MovementHash>,
		pre_image: HashLockPreImage,
	) -> SCCResult<MovementAddress, MovementHash> {
		let transfer = self
			.locked_transfers
			.remove(bridge_transfer_id)
			.ok_or(SmartContractCounterpartyError::TransferNotFound)?;

		tracing::trace!("SmartContractCounterparty: Completing bridge transfer: {:?}", transfer);

		// check if the secret is correct
		let secret_hash = MovementHash::from(pre_image.clone());
		if transfer.hash_lock.0 != secret_hash {
			tracing::warn!(
				"Invalid hash lock pre image {pre_image:?} hash {secret_hash:?} != hash_lock {:?}",
				transfer.hash_lock.0
			);
			return Err(SmartContractCounterpartyError::InvalidHashLockPreImage);
		}

		// TODO: fix this
		let account = MovementAddress::from(transfer.recipient_address.clone());

		let balance = accounts.entry(account).or_insert(Amount(AssetType::EthAndWeth((0, 0))));
		// balance += **transfer.amount;

		Ok(SmartContractCounterpartyEvent::CompletedBridgeTransfer(
			CounterpartyCompletedDetails::from_lock_details(transfer, pre_image),
		))
	}
}

#[async_trait::async_trait]
impl BridgeContractCounterparty for MovementSmartContractCounterparty {
	type Address = MovementAddress;
	type Hash = MovementHash;

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
