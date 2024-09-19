use crate::counterparty_contract::EthSmartContractCounterparty;
use crate::initiator_contract::EthSmartContractInitiator;
use bridge_shared::{
	blockchain_service::{BlockchainService, ContractEvent},
	bridge_monitoring::BridgeContractInitiatorEvent,
	types::{Amount, CounterpartyCall, InitiatorCall, SmartContractInitiatorEvent},
};
use client::{Config, EthClient};
use event_monitoring::{EthCounterpartyMonitoring, EthInitiatorMonitoring};
use event_types::EthChainEvent;
use futures::{
	channel::mpsc::{self},
	task::AtomicWaker,
	Stream, StreamExt,
};
use std::fmt::Debug;
use std::{
	collections::HashMap,
	future::Future,
	pin::Pin,
	task::{Context, Poll},
};
use types::{EthAddress, EthHash};

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

pub struct EthereumChain {
	pub name: String,
	pub time: u64,
	pub accounts: HashMap<EthAddress, Amount>,
	pub events: Vec<EthChainEvent<EthAddress, EthHash>>,

	pub initiator_contract: EthSmartContractInitiator,
	pub initiator_monitoring: EthInitiatorMonitoring<EthAddress, EthHash>,
	pub counterparty_contract: EthSmartContractCounterparty,
	pub counterparty_monitoring: EthCounterpartyMonitoring<EthAddress, EthHash>,

	pub transaction_sender: mpsc::UnboundedSender<Transaction<EthAddress, EthHash>>,
	pub transaction_receiver: mpsc::UnboundedReceiver<Transaction<EthAddress, EthHash>>,

	pub event_listeners: Vec<mpsc::UnboundedSender<EthChainEvent<EthAddress, EthHash>>>,

	waker: AtomicWaker,
	pub _phantom: std::marker::PhantomData<EthHash>,
}

impl EthereumChain {
	pub async fn new(name: impl Into<String>, rpc_url: &str) -> Self {
		let accounts = HashMap::new();
		let events = Vec::new();
		let (_, event_receiver_1) = mpsc::unbounded();
		let (_, event_receiver_2) = mpsc::unbounded();
		let (event_sender, event_receiver) = mpsc::unbounded();
		let event_listeners = Vec::new();

		Self {
			name: name.into(),
			time: 0,
			accounts,
			events,
			initiator_contract: EthSmartContractInitiator::new(),
			initiator_monitoring: EthInitiatorMonitoring::build(rpc_url, event_receiver_1)
				.await
				.expect("Failed to create EthInitiatorMonitoring"),
			counterparty_contract: EthSmartContractCounterparty::new(),
			counterparty_monitoring: EthCounterpartyMonitoring::build(rpc_url, event_receiver_2)
				.await
				.expect("Failed to create EthCounterpartyMonitoring"),
			transaction_sender: event_sender,
			transaction_receiver: event_receiver,
			event_listeners,
			waker: AtomicWaker::new(),
			_phantom: std::marker::PhantomData,
		}
	}

	pub fn add_event_listener(
		&mut self,
	) -> mpsc::UnboundedReceiver<EthChainEvent<EthAddress, EthHash>> {
		let (sender, receiver) = mpsc::unbounded();
		self.event_listeners.push(sender);
		receiver
	}

	pub fn add_account(&mut self, address: EthAddress, amount: Amount) {
		self.accounts.insert(address, amount);
	}

	pub fn get_balance(&mut self, address: &EthAddress) -> Option<&Amount> {
		self.accounts.get(address)
	}

	pub fn connection(&self) -> mpsc::UnboundedSender<Transaction<EthAddress, EthHash>> {
		self.transaction_sender.clone()
	}

	pub async fn client(config: Config) -> EthClient {
		EthClient::new(config).await.expect("failed to create client")
	}
}

impl Future for EthereumChain {
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

impl Stream for EthereumChain {
	type Item = ContractEvent<EthAddress, EthHash>;

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
			match event {
				EthChainEvent::InitiatorContractEvent(Ok(event)) => {
					let contract_event = match event {
						SmartContractInitiatorEvent::InitiatedBridgeTransfer(details) => {
							BridgeContractInitiatorEvent::Initiated(details)
						}
						SmartContractInitiatorEvent::CompletedBridgeTransfer(transfer_id) => {
							BridgeContractInitiatorEvent::Completed(transfer_id)
						}
						SmartContractInitiatorEvent::RefundedBridgeTransfer(transfer_id) => {
							BridgeContractInitiatorEvent::Refunded(transfer_id)
						}
					};
					return Poll::Ready(Some(ContractEvent::InitiatorEvent(contract_event)));
				}
				EthChainEvent::InitiatorContractEvent(Err(_)) => {
					// trace here
					return Poll::Ready(None);
				}
				EthChainEvent::CounterpartyContractEvent(_) => {
					// trace here
					return Poll::Ready(None);
				}
				EthChainEvent::Noop => {
					//trace here
					return Poll::Ready(None);
				}
			}
		}

		tracing::trace!("AbstractBlockchain[{}]: Poll::Pending", this.name);
		Poll::Pending
	}
}

impl BlockchainService for EthereumChain {
	type Address = EthAddress;
	type Hash = EthHash;

	// InitiatorContract must be BridgeContractInitiator
	// These are just the Client Structs!!
	type InitiatorContract = EthSmartContractInitiator;
	type InitiatorMonitoring = EthInitiatorMonitoring<EthAddress, EthHash>;

	type CounterpartyContract = EthSmartContractCounterparty;
	type CounterpartyMonitoring = EthCounterpartyMonitoring<EthAddress, EthHash>;

	fn initiator_contract(&self) -> &Self::InitiatorContract {
		&self.initiator_contract
	}

	fn initiator_monitoring(&mut self) -> &mut Self::InitiatorMonitoring {
		&mut self.initiator_monitoring
	}

	fn counterparty_contract(&self) -> &Self::CounterpartyContract {
		&self.counterparty_contract
	}

	fn counterparty_monitoring(&mut self) -> &mut Self::CounterpartyMonitoring {
		&mut self.counterparty_monitoring
	}
}
