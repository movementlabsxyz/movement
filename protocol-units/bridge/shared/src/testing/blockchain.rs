use futures::Stream;
use std::{
	collections::HashMap,
	pin::Pin,
	task::{Context, Poll},
};

use super::rng::RngSeededClone;
use crate::types::{
	Amount, BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId,
	GenUniqueHash, HashLock, InitiatorAddress, LockedAssetsDetails, RecipientAddress, TimeLock,
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

#[derive(Debug, Clone)]
pub enum AbstractBlockchainEvent {
	Noop,
}

#[derive(Debug)]
pub struct AbstractBlockchain<A, H, R> {
	pub name: String,
	pub time: u64,
	pub accounts: HashMap<A, Amount>,
	pub events: Vec<AbstractBlockchainEvent>,
	pub rng: R,

	pub initiater_contract: SmartContractInitiator<A, H>,
	pub counterparty_contract: SmartContractCounterparty<A, H>,

	pub _phantom: std::marker::PhantomData<H>,
}

impl<A, H, R> AbstractBlockchain<A, H, R>
where
	A: BridgeAddressType,
	H: BridgeHashType + GenUniqueHash,
	R: RngSeededClone,
{
	pub fn new(rng: R, name: impl Into<String>) -> Self {
		let accounts = HashMap::new();
		let events = Vec::new();

		Self {
			name: name.into(),
			time: 0,
			accounts,
			events,
			rng,
			initiater_contract: SmartContractInitiator::new(),
			counterparty_contract: SmartContractCounterparty::new(),
			_phantom: std::marker::PhantomData,
		}
	}

	pub fn forward_time(&mut self, duration: u64) {
		self.time += duration;
	}

	pub fn add_account(&mut self, address: A, amount: Amount) {
		self.accounts.insert(address, amount);
	}

	pub fn get_balance(&mut self, address: &A) -> Option<Amount> {
		self.accounts.get(address).cloned()
	}
}

impl<A, H, R> Stream for AbstractBlockchain<A, H, R>
where
	A: BridgeAddressType,
	H: BridgeHashType,
	R: Unpin,
{
	type Item = AbstractBlockchainEvent;

	fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Some(event) = this.events.pop() {
			Poll::Ready(Some(event))
		} else {
			Poll::Pending
		}
	}
}
