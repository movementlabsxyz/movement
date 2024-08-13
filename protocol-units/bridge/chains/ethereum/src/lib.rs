use std::{
	collections::HashMap,
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};

use bridge_shared::{
	initiator_contract::{SCIResult, SmartContractInitiator},
	types::{
		Amount, BridgeAddressType, BridgeHashType, GenUniqueHash, HashLockPreImage,
		RecipientAddress,
	},
};
use futures::{channel::mpsc, task::AtomicWaker, Stream, StreamExt};
use types::CounterpartyCall;
use utils::RngSeededClone;

mod client;
mod event_logging;
mod event_types;
mod types;
mod utils;

pub enum SmartContractCall<A, H> {
	Initiator(),
	Counterparty(CounterpartyCall<A, H>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthChainEvent<A, H> {
	InitiatorContractEvent(SCIResult<A, H>),
	CounterpartyContractEvent(SCCResult<A, H>),
	Noop,
}

#[derive(Debug)]
pub enum Transaction<A, H> {
	Initiator(InitiatorCall<A, H>),
	Counterparty(CounterpartyCall<A, H>),
}

pub struct EthereumChain<A, H, R> {
	pub name: String,
	pub time: u64,
	pub accounts: HashMap<A, Amount>,
	pub events: Vec<EthChainEvent<A, H>>,

	pub initiator_contract: SmartContractInitiator<A, H, R>,
	pub counterparty_contract: SmartContractCounterparty<A, H>,

	pub transaction_sender: mpsc::UnboundedSender<EthTransaction<A, H>>,
	pub transaction_receiver: mpsc::UnboundedReceiver<EthTransaction<A, H>>,

	pub event_listeners: Vec<mpsc::UnboundedSender<EthTransaction<A, H>>>,

	waker: AtomicWaker,

	pub _phantom: std::marker::PhantomData<H>,
}

impl<A, H, R> EthereumChain<A, H, R>
where
	A: BridgeAddressType + From<RecipientAddress<A>>,
	H: BridgeHashType + GenUniqueHash,
	R: RngSeededClone,
	H: From<HashLockPreImage>,
{
	pub fn new(mut rng: R, name: impl Into<String>) -> Self {
		let accounts = HashMap::new();
		let events = Vec::new();
		let (event_sender, event_receiver) = mpsc::unbounded();
		let event_listeners = Vec::new();

		Self {
			name: name.into(),
			time: 0,
			accounts,
			events,
			initiator_contract: SmartContractInitiator::new(rng.seeded_clone()),
			counterparty_contract: SmartContractCounterparty::new(),
			transaction_sender: event_sender,
			transaction_receiver: event_receiver,
			event_listeners,
			waker: AtomicWaker::new(),
			_phantom: std::marker::PhantomData,
		}
	}

	pub fn add_event_listener(&mut self) -> mpsc::UnboundedReceiver<EthTransaction<A, H>> {
		let (sender, receiver) = mpsc::unbounded();
		self.event_listeners.push(sender);
		receiver
	}

	pub fn add_account(&mut self, address: A, amount: Amount) {
		self.accounts.insert(address, amount);
	}

	pub fn get_balance(&mut self, address: &A) -> Option<&Amount> {
		self.accounts.get(address)
	}

	pub fn connection(&self) -> mpsc::UnboundedSender<EthTransaction<A, H>> {
		self.transaction_sender.clone()
	}
}

impl<A, H, R> Future for EthereumChain<A, H, R>
where
	A: BridgeAddressType + From<RecipientAddress<A>>,
	H: BridgeHashType + GenUniqueHash,
	R: RngSeededClone + Unpin,
	H: From<HashLockPreImage>,
{
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

impl<A, H, R> Stream for EthereumChain<A, H, R>
where
	A: BridgeAddressType + From<RecipientAddress<A>>,
	H: BridgeHashType + GenUniqueHash + From<HashLockPreImage>,
	R: RngSeededClone + Unpin,
{
	type Item = EthChainEvent<A, H>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		tracing::trace!("AbstractBlockchain[{}]: Polling for events", self.name);
		let this = self.get_mut();

		match this.transaction_receiver.poll_next_unpin(cx) {
			Poll::Ready(Some(transaction)) => {
				tracing::trace!(
					"AbstractBlockchain[{}]: Received transaction: {:?}",
					this.name,
					transaction
				);
				match transaction {
					EthTransaction::Initiator(call) => match call {
						InitiatorCall::InitiateBridgeTransfer(
							initiator_address,
							recipient_address,
							amount,
							time_lock,
							hash_lock,
						) => {
							this.events.push(EthChainEvent::InitiatorContractEvent(
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
							this.events.push(EthChainEvent::InitiatorContractEvent(
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
							this.events.push(EthChainEvent::CounterpartyContractEvent(
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
							this.events.push(EthChainEvent::CounterpartyContractEvent(
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
