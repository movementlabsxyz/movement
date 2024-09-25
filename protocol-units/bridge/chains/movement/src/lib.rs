use bridge_shared::types::{Amount, CounterpartyCall, InitiatorCall};
use client::{Config, MovementClient};
use event_monitoring::{MovementCounterpartyMonitoring, MovementInitiatorMonitoring};
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
pub mod event_monitoring;
pub mod event_types;
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

	pub initiator_contract: MovementClient,
	pub initiator_monitoring: MovementInitiatorMonitoring<MovementAddress, MovementHash>,
	pub counterparty_contract: MovementClient,
	pub counterparty_monitor: MovementCounterpartyMonitoring<MovementAddress, MovementHash>,

	pub transaction_sender:
		mpsc::UnboundedSender<MovementChainEvent<MovementAddress, MovementHash>>,
	pub transaction_receiver:
		mpsc::UnboundedReceiver<MovementChainEvent<MovementAddress, MovementHash>>,

	pub event_listeners:
		Vec<mpsc::UnboundedSender<MovementChainEvent<MovementAddress, MovementHash>>>,

	waker: AtomicWaker,

	pub _phantom: std::marker::PhantomData<MovementHash>,
}

impl MovementChain {
	pub async fn new() -> Self {
		let accounts = HashMap::new();
		let events = Vec::new();
		let (_, event_receiver_1) = mpsc::unbounded();
		let (_, event_receiver_2) = mpsc::unbounded();
		let (event_sender, event_receiver_3) = mpsc::unbounded();
		let event_listeners = Vec::new();
		//
		//TODO: Should be configurable via static json files
		let config = Config::build_for_test();

		let (client, _) =
			MovementClient::new_for_test(config).await.expect("Failed to create client");

		Self {
			name: "MovementChain".to_string(),
			time: 0,
			accounts,
			events,
			initiator_contract: client.clone(),
			initiator_monitoring: MovementInitiatorMonitoring::build(
				"localhost:8545",
				event_receiver_1,
			)
			.await
			.expect("Failed to create client"),
			counterparty_contract: client.clone(),
			counterparty_monitor: MovementCounterpartyMonitoring::build(
				"localhost:8080",
				event_receiver_2,
			)
			.await
			.expect("Failed to create client"),
			transaction_sender: event_sender,
			transaction_receiver: event_receiver_3,
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

	pub fn connection(
		&self,
	) -> mpsc::UnboundedSender<MovementChainEvent<MovementAddress, MovementHash>> {
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
					_ => {} //Implement chain event logic here
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
