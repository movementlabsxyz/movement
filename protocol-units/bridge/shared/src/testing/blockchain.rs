use futures::Stream;
use std::{
	collections::HashMap,
	pin::Pin,
	task::{Context, Poll},
};

use self::{
	counterparty_contract::SmartContractCounterparty, initiator_contract::SmartContractInitiator,
};

use super::rng::RngSeededClone;
use crate::types::{Amount, BridgeAddressType, BridgeHashType, GenUniqueHash};

pub mod counterparty_contract;
pub mod initiator_contract;

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
