use futures::{channel::mpsc, Stream, StreamExt};
use std::{
	collections::HashMap,
	pin::Pin,
	task::{Context, Poll},
};

use self::{
	counterparty_contract::{CounterpartyCall, SmartContractCounterparty},
	initiator_contract::{InitiatorCall, SmartContractInitiator},
};

use super::rng::RngSeededClone;
use crate::types::{Amount, BridgeAddressType, BridgeHashType, GenUniqueHash};

pub mod counterparty_contract;
pub mod initiator_contract;

#[derive(Debug, Clone)]
pub enum AbstractBlockchainEvent {
	Noop,
}

pub enum Transaction<A, H> {
	Initiator(InitiatorCall<A, H>),
	Counterparty(CounterpartyCall<A, H>),
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

	pub transaction_sender: mpsc::UnboundedSender<Transaction<A, H>>,
	pub transaction_receiver: mpsc::UnboundedReceiver<Transaction<A, H>>,

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
		let (event_sender, event_receiver) = mpsc::unbounded();

		Self {
			name: name.into(),
			time: 0,
			accounts,
			events,
			rng,
			initiater_contract: SmartContractInitiator::new(),
			counterparty_contract: SmartContractCounterparty::new(),
			transaction_sender: event_sender,
			transaction_receiver: event_receiver,
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

	pub fn connection(&self) -> mpsc::UnboundedSender<Transaction<A, H>> {
		self.transaction_sender.clone()
	}

	pub fn client(&self) -> AbstractBlockchainClient<A, H> {
		AbstractBlockchainClient { transaction_sender: self.transaction_sender.clone() }
	}
}

pub struct AbstractBlockchainClient<A, H> {
	pub transaction_sender: mpsc::UnboundedSender<Transaction<A, H>>,
}

impl<A, H> AbstractBlockchainClient<A, H> {
	pub fn send_transaction(&self, transaction: Transaction<A, H>) {
		self.transaction_sender
			.unbounded_send(transaction)
			.expect("Failed to send transaction");
	}
}

impl<A, H, R> Stream for AbstractBlockchain<A, H, R>
where
	A: BridgeAddressType,
	H: BridgeHashType + GenUniqueHash,
	R: Unpin,
{
	type Item = AbstractBlockchainEvent;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		if let Poll::Ready(Some(transaction)) = this.transaction_receiver.poll_next_unpin(cx) {
			match transaction {
				Transaction::Initiator(call) => match call {
					InitiatorCall::InitiateBridgeTransfer(
						initiator_address,
						recipient_address,
						amount,
						time_lock,
						hash_lock,
					) => {
						this.initiater_contract.initiate_bridge_transfer(
							initiator_address,
							recipient_address,
							amount,
							time_lock,
							hash_lock,
						);
					}
				},
				Transaction::Counterparty(call) => match call {
					CounterpartyCall::LockBridgeTransfer(
						bridge_transfer_id,
						hash_lock,
						time_lock,
						recipient_address,
						amount,
					) => {
						this.counterparty_contract.lock_bridge_transfer(
							bridge_transfer_id,
							hash_lock,
							time_lock,
							recipient_address,
							amount,
						);
					}
				},
			}
		}

		if let Some(event) = this.events.pop() {
			return Poll::Ready(Some(event));
		}

		Poll::Pending
	}
}
