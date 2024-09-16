use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use bridge_shared::{
	bridge_contracts::{BridgeContractInitiator, BridgeContractInitiatorResult},
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, HashLockPreImage,
		InitiatorAddress, RecipientAddress, TimeLock,
	},
};

use crate::utils::{MovementAddress, MovementHash};

#[derive(Debug, Clone)]
pub struct MovementSmartContractInitiator {
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
