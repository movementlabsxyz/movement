use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use bridge_shared::{
	bridge_contracts::{BridgeContractInitiator, BridgeContractInitiatorResult},
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
		InitiatorAddress, RecipientAddress, SCIResult, SmartContractInitiatorEvent, TimeLock,
	},
};

use crate::utils::{MovementAddress, MovementHash};

#[derive(Debug, Clone)]
pub struct MovementSmartContractInitiator {
	#[allow(clippy::type_complexity)]
	pub initiated_transfers: Arc<
		RwLock<
			HashMap<
				BridgeTransferId<MovementHash>,
				BridgeTransferDetails<MovementAddress, MovementHash>,
			>,
		>,
	>,
	pub accounts: HashMap<MovementAddress, Amount>,
}

impl MovementSmartContractInitiator {
	pub fn new() -> Self {
		Self {
			initiated_transfers: Arc::new(RwLock::new(HashMap::new())),
			accounts: HashMap::default(),
		}
	}

	pub fn initiate_bridge_transfer(
		&mut self,
		initiator: InitiatorAddress<MovementAddress>,
		recipient: RecipientAddress<Vec<u8>>,
		amount: Amount,
		time_lock: TimeLock,
		hash_lock: HashLock<MovementHash>,
	) -> SCIResult<MovementAddress, MovementHash> {
		// Update balance (you might need to adjust this logic based on your requirements)
		// let balance = self.accounts.entry(initiator.0.clone()).or_insert(Amount(0));
		// *balance -= amount.weth();

		// Lock the RwLock and get a mutable reference to the `initiated_transfers`
		let mut initiated_transfers = self.initiated_transfers.write().unwrap();
		let dummy_id = BridgeTransferId(MovementHash::random());

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
		_accounts: &mut HashMap<MovementAddress, Amount>,
		transfer_id: BridgeTransferId<MovementHash>,
		_pre_image: HashLockPreImage,
	) -> SCIResult<MovementAddress, MovementHash> {
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
impl BridgeContractInitiator for MovementSmartContractInitiator {
	type Address = MovementAddress;
	type Hash = MovementHash;

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