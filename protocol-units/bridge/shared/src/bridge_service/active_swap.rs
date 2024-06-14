use std::{collections::HashMap, pin::Pin};

use futures::Future;

use crate::types::{BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId};

pub type BoxedBridgeServiceFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Bridge state in the sense of tracking the active swaps from the bridge
pub enum ActiveSwap<BFromA, BFromH> {
	/// Bridge is locking lockens on the counterpart chain
	LockingTokens(BridgeTransferDetails<BFromA, BFromH>, BoxedBridgeServiceFuture),
	/// Bridge is waiting for the initiator to complete her transfer
	/// revealing her secret.
	WaitingForCompletedEvent(BridgeTransferId<BFromH>),
	/// We are in possession of the secret and are now completing the bridge tranfer on our side
	CompletingBridging(BoxedBridgeServiceFuture),
	/// We have completed the atomic bdridge transfer
	Completed,
}

pub struct ActiveSwapMap<BFromA, BFromH, BFromCI, BToCC> {
	pub initiator_contract: BFromCI,
	pub counterparty_contract: BToCC,
	swaps: HashMap<BridgeTransferId<BFromH>, ActiveSwap<BFromA, BFromH>>,
}

impl<BFromA, BFromH, BFromCI, BToCC> ActiveSwapMap<BFromA, BFromH, BFromCI, BToCC>
where
	BFromH: BridgeHashType,
	BFromA: BridgeAddressType,
{
	pub fn build(initiator_contract: BFromCI, counterparty_contract: BToCC) -> Self {
		Self { initiator_contract, counterparty_contract, swaps: HashMap::new() }
	}

	pub fn get(&self, key: &BridgeTransferId<BFromH>) -> Option<&ActiveSwap<BFromA, BFromH>> {
		self.swaps.get(key)
	}

	pub fn already_executing(&self, key: &BridgeTransferId<BFromH>) -> bool {
		self.swaps.contains_key(key)
	}

	pub fn start(&mut self, details: BridgeTransferDetails<BFromA, BFromH>) {
		assert!(self.swaps.get(&details.bridge_transfer_id).is_none());

		self.swaps.insert(
			details.bridge_transfer_id.clone(),
			ActiveSwap::LockingTokens(details, Box::pin(async move {})),
		);
	}
}
