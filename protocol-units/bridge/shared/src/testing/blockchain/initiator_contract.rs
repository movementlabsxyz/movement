use std::collections::HashMap;

use crate::types::{
	Amount, BridgeHashType, BridgeTransferDetails, BridgeTransferId, GenUniqueHash, HashLock,
	InitiatorAddress, RecipientAddress, TimeLock,
};

#[derive(Debug)]
pub struct SmartContractInitiator<A, H> {
	pub initiated_transfers: HashMap<BridgeTransferId<H>, BridgeTransferDetails<A, H>>,
}

impl<A, H> Default for SmartContractInitiator<A, H>
where
	H: BridgeHashType + GenUniqueHash,
{
	fn default() -> Self {
		Self::new()
	}
}

impl<A, H> SmartContractInitiator<A, H>
where
	H: BridgeHashType + GenUniqueHash,
{
	pub fn new() -> Self {
		Self { initiated_transfers: HashMap::new() }
	}

	pub fn initiate_bridge_transfer(
		&mut self,
		initiator: InitiatorAddress<A>,
		recipient: RecipientAddress<A>,
		amount: Amount,
		time_lock: TimeLock,
		hash_lock: HashLock<H>,
	) {
		let bridge_tranfer_id = BridgeTransferId::<H>::gen_unique_hash();
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
}
