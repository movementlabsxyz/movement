use crate::counterparty_contract::MovementSmartContractCounterparty;
use crate::initiator_contract::MovementSmartContractInitiator;
use bridge_shared::types::{
	Amount, BridgeAddressType, BridgeHashType, CounterpartyCall, GenUniqueHash, HashLockPreImage,
	InitiatorCall, RecipientAddress,
};
use event_types::MovementChainEvent;
use futures::{channel::mpsc, task::AtomicWaker, Stream, StreamExt};
use std::{
	collections::HashMap,
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};
use utils::{MovementAddress, MovementHash};

pub mod client;
pub mod counterparty_contract;
pub mod event_monitoring;
pub mod event_types;
pub mod initiator_contract;
pub mod types;
pub mod utils;

pub enum SmartContractCall<A, H> {
	Initiator(),
	Counterparty(CounterpartyCall<A, H>),
}

/// A Bridge Transaction that can occur on any supported network.
#[derive(Debug)]
pub enum Transaction<A, H> {
	Initiator(InitiatorCall<A, H>),
	Counterparty(CounterpartyCall<A, H>),
}

#[allow(unused)]
pub struct MovementChain {
	pub name: String,
	pub time: u64,
	pub accounts: HashMap<MovementAddress, Amount>,
	pub events: Vec<MovementChainEvent<MovementAddress, MovementHash>>,

	pub initiator_contract: MovementSmartContractInitiator,
	pub counterparty_contract: MovementSmartContractCounterparty,

	pub transaction_sender: mpsc::UnboundedSender<Transaction<MovementAddress, MovementHash>>,
	pub transaction_receiver: mpsc::UnboundedReceiver<Transaction<MovementAddress, MovementHash>>,

	pub event_listeners:
		Vec<mpsc::UnboundedSender<MovementChainEvent<MovementAddress, MovementHash>>>,

	waker: AtomicWaker,

	pub _phantom: std::marker::PhantomData<MovementHash>,
}

impl MovementChain {
	pub fn new() -> Self {
		let accounts = HashMap::new();
		let events = Vec::new();
		let (event_sender, event_receiver) = mpsc::unbounded();
		let event_listeners = Vec::new();

		Self {
			name: "MovementChain".to_string(),
			time: 0,
			accounts,
			events,
			initiator_contract: MovementSmartContractInitiator::new(),
			counterparty_contract: MovementSmartContractCounterparty::new(),
			transaction_sender: event_sender,
			transaction_receiver: event_receiver,
			event_listeners,
			waker: AtomicWaker::new(),
			_phantom: std::marker::PhantomData,
		}
	}

	pub fn add_event_listener(
		&mut self,
	) -> mpsc::UnboundedReceiver<MovementChainEvent<MovementAddress, MovementHash>> {
		let (sender, receiver) = mpsc::unbounded();
		self.event_listeners.push(sender);
		receiver
	}

	pub fn add_account(&mut self, address: MovementAddress, amount: Amount) {
		self.accounts.insert(address, amount);
	}

	pub fn get_balance(&mut self, address: &MovementAddress) -> Option<&Amount> {
		self.accounts.get(address)
	}

	pub fn connection(&self) -> mpsc::UnboundedSender<Transaction<MovementAddress, MovementHash>> {
		self.transaction_sender.clone()
	}
}

impl Future for MovementChain {
	type Output = ();

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let this = self.get_mut();

		// This simulates
		// async move { while (blockchain_1.next().await).is_some() {} }
		while let Poll::Ready(event) = this.poll_next_unpin(cx) {
			match event {
				Some(_) => {}
				None => return Poll::Ready(()),
			}
		}
		Poll::Pending
	}
}

impl Stream for MovementChain {
	type Item = MovementChainEvent<MovementAddress, MovementHash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		tracing::trace!("AbstractBlockchain[{}]: Polling for events", self.name);
		let this = self.get_mut();

		match this.transaction_receiver.poll_next_unpin(cx) {
			Poll::Ready(Some(transaction)) => {
				tracing::trace!(
					"Etherum Chain [{}]: Received transaction: {:?}",
					this.name,
					transaction
				);
				match transaction {
					Transaction::Initiator(call) => match call {
						InitiatorCall::InitiateBridgeTransfer(
							initiator_address,
							recipient_address,
							amount,
							time_lock,
							hash_lock,
						) => {
							this.events.push(MovementChainEvent::InitiatorContractEvent(
								this.initiator_contract.initiate_bridge_transfer(
									initiator_address.clone(),
									recipient_address.clone(),
									amount,
									time_lock.clone(),
									hash_lock.clone(),
								),
							));
						}
						InitiatorCall::CompleteBridgeTransfer(bridge_transfer_id, secret) => {
							this.events.push(MovementChainEvent::InitiatorContractEvent(
								this.initiator_contract.complete_bridge_transfer(
									&mut this.accounts,
									bridge_transfer_id.clone(),
									secret.clone(),
								),
							));
						}
					},
					Transaction::Counterparty(call) => match call {
						CounterpartyCall::LockBridgeTransfer(
							bridge_transfer_id,
							hash_lock,
							time_lock,
							initiator_address,
							recipient_address,
							amount,
						) => {
							this.events.push(MovementChainEvent::CounterpartyContractEvent(
								this.counterparty_contract.lock_bridge_transfer(
									bridge_transfer_id.clone(),
									hash_lock.clone(),
									time_lock.clone(),
									initiator_address.clone(),
									recipient_address.clone(),
									amount,
								),
							));
						}
						CounterpartyCall::CompleteBridgeTransfer(bridge_transfer_id, pre_image) => {
							this.events.push(MovementChainEvent::CounterpartyContractEvent(
								this.counterparty_contract.complete_bridge_transfer(
									&mut this.accounts,
									&bridge_transfer_id,
									pre_image,
								),
							));
						}
					},
				}
			}
			Poll::Ready(None) => {
				tracing::warn!("AbstractBlockchain[{}]: Transaction receiver dropped", this.name);
			}
			Poll::Pending => {
				tracing::trace!(
					"AbstractBlockchain[{}]: No events in transaction_receiver",
					this.name
				);
			}
		}

		if let Some(event) = this.events.pop() {
			for listener in &mut this.event_listeners {
				tracing::trace!("AbstractBlockchain[{}]: Sending event to listener", this.name);
				listener.unbounded_send(event.clone()).expect("listener dropped");
			}

			tracing::trace!("AbstractBlockchain[{}]: Poll::Ready({:?})", this.name, event);
			return Poll::Ready(Some(event));
		}

		tracing::trace!("AbstractBlockchain[{}]: Poll::Pending", this.name);
		Poll::Pending
	}
}
