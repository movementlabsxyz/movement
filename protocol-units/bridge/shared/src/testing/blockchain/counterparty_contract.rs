use std::collections::HashMap;

use crate::types::{
	Amount, BridgeHashType, BridgeTransferId, GenUniqueHash, HashLock, LockedAssetsDetails,
	RecipientAddress, TimeLock,
};

#[derive(Debug)]
pub struct SmartContractCounterparty<A, H> {
	pub locked_transfers: HashMap<BridgeTransferId<H>, LockedAssetsDetails<A, H>>,
}

impl<A, H> Default for SmartContractCounterparty<A, H>
where
	H: BridgeHashType + GenUniqueHash,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<A, H> SmartContractCounterparty<A, H>
where
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
		self.locked_transfers.insert(
			bridge_transfer_id.clone(),
			LockedAssetsDetails {
				bridge_transfer_id,
				recipient_address,
				hash_lock,
				time_lock,
				amount,
			},
		);
	}
}
