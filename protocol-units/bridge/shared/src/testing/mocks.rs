use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::bridge_monitoring::{
	BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
	BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
};
use crate::types::{BridgeTransferDetails, BridgeTransferId};
use crate::{
	blockchain_service::{BlockchainEvent, BlockchainService},
	types::{HashLock, InitiatorAddress, RecipientAddress, TimeLock},
};
use crate::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator, BridgeContractResult},
	types::Amount,
};

pub struct MockBlockchainService<A, H> {
	pub initiator_contract: MockInitiatorContract<A, H>,
	pub initiator_monitoring: MockInitiatorMonitoring<A, H>,
	pub counterparty_contract: MockCounterpartyContract<A, H>,
	pub counterparty_monitoring: MockCounterpartyMonitoring<A, H>,
}

impl<A, H> MockBlockchainService<A, H> {
	pub fn build() -> Self {
		Self {
			initiator_contract: MockInitiatorContract::build(),
			initiator_monitoring: MockInitiatorMonitoring::build(),
			counterparty_contract: MockCounterpartyContract::build(),
			counterparty_monitoring: MockCounterpartyMonitoring::build(),
		}
	}
}

impl<A, H> BlockchainService for MockBlockchainService<A, H>
where
	A: std::fmt::Debug + Unpin + Send + Sync,
	H: std::fmt::Debug + Unpin + Send + Sync + Clone,
{
	type Address = A;
	type Hash = H;

	type InitiatorContract = MockInitiatorContract<A, H>;
	type InitiatorMonitoring = MockInitiatorMonitoring<A, H>;

	type CounterpartyContract = MockCounterpartyContract<A, H>;
	type CounterpartyMonitoring = MockCounterpartyMonitoring<A, H>;

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

impl<A, H> Stream for MockBlockchainService<A, H>
where
	A: std::fmt::Debug + Unpin + Send + Sync,
	H: std::fmt::Debug + Unpin + Send + Sync + Clone,
{
	type Item =
		BlockchainEvent<<Self as BlockchainService>::Address, <Self as BlockchainService>::Hash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		if let Poll::Ready(Some(event)) = this.poll_next_event(cx) {
			return Poll::Ready(Some(event));
		}

		// poll the events from the contracts and forward them
		if let Poll::Ready(Some(event)) = this.initiator_contract.poll_next_unpin(cx) {
			return Poll::Ready(Some(BlockchainEvent::InitiatorEvent(event)));
		}

		Poll::Pending
	}
}

pub struct MockInitiatorMonitoring<A, H> {
	pub events: Vec<BridgeContractInitiatorEvent<A, H>>,
}

impl<A, H> MockInitiatorMonitoring<A, H> {
	pub fn build() -> Self {
		Self { events: Default::default() }
	}
}

impl<A, H> BridgeContractInitiatorMonitoring for MockInitiatorMonitoring<A, H>
where
	A: std::fmt::Debug + Unpin,
	H: std::fmt::Debug + Unpin,
{
	type Address = A;
	type Hash = H;
}

impl<A, H> Stream for MockInitiatorMonitoring<A, H>
where
	A: std::fmt::Debug + Unpin,
	H: std::fmt::Debug + Unpin,
{
	type Item = BridgeContractInitiatorEvent<A, H>;

	fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Some(event) = this.events.pop() {
			Poll::Ready(Some(event))
		} else {
			Poll::Pending
		}
	}
}

pub struct MockCounterpartyMonitoring<A, H> {
	pub events: Vec<BridgeContractCounterpartyEvent<A, H>>,
}

impl<A, H> MockCounterpartyMonitoring<A, H> {
	pub fn build() -> Self {
		Self { events: Default::default() }
	}
}

impl<A, H> BridgeContractCounterpartyMonitoring for MockCounterpartyMonitoring<A, H>
where
	A: std::fmt::Debug + Unpin,
	H: std::fmt::Debug + Unpin,
{
	type Address = A;
	type Hash = H;
}

impl<A, H> Stream for MockCounterpartyMonitoring<A, H>
where
	A: std::fmt::Debug + Unpin,
	H: std::fmt::Debug + Unpin,
{
	type Item = BridgeContractCounterpartyEvent<A, H>;

	fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Some(event) = this.events.pop() {
			Poll::Ready(Some(event))
		} else {
			Poll::Pending
		}
	}
}

#[derive(Debug)]
pub struct MockInitiatorContract<A, H> {
	events: Vec<BridgeContractInitiatorEvent<A, H>>,
	_phantom: std::marker::PhantomData<(A, H)>,
}

impl<A, H> Stream for MockInitiatorContract<A, H>
where
	A: Unpin,
	H: Unpin,
{
	type Item = BridgeContractInitiatorEvent<A, H>;

	fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Some(event) = this.events.pop() {
			Poll::Ready(Some(event))
		} else {
			Poll::Pending
		}
	}
}

impl<A, H> MockInitiatorContract<A, H> {
	pub fn build() -> Self {
		Self { events: Default::default(), _phantom: std::marker::PhantomData }
	}
}

#[async_trait::async_trait]
impl<A, H> BridgeContractInitiator for MockInitiatorContract<A, H>
where
	A: std::fmt::Debug + Send + Sync,
	H: std::fmt::Debug + Send + Sync + Clone,
{
	type Address = A;
	type Hash = H;

	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Self::Address>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractResult<()> {
		self.events.push(BridgeContractInitiatorEvent::BridgeTransferInitiated(
			BridgeTransferDetails {
				bridge_transfer_id: BridgeTransferId(Clone::clone(&*hash_lock)),
				initiator_address,
				recipient_address,
				hash_lock,
				time_lock,
				amount,
			},
		));
		Ok(())
	}

	async fn complete_bridge_transfer<S: Send>(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: S,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn refund_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		Ok(None)
	}
}

pub struct MockCounterpartyContract<A, H> {
	_phantom: std::marker::PhantomData<(A, H)>,
}

impl<A, H> MockCounterpartyContract<A, H> {
	pub fn build() -> Self {
		Self { _phantom: std::marker::PhantomData }
	}
}

#[async_trait::async_trait]
impl<A, H> BridgeContractCounterparty for MockCounterpartyContract<A, H>
where
	A: std::fmt::Debug + Send + Sync,
	H: std::fmt::Debug + Send + Sync,
{
	type Address = A;
	type Hash = H;

	async fn lock_bridge_transfer_assets(
		&self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_hash_lock: HashLock<Self::Hash>,
		_time_lock: TimeLock,
		_recipient: RecipientAddress<Self::Address>,
		_amount: Amount,
	) -> bool {
		true
	}

	async fn complete_bridge_transfer<S: Send>(
		&self,
		_bridge_transfer_id: Self::Hash,
		_secret: S,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn abort_bridge_transfer(
		&self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&self,
		_bridge_transfer_id: Self::Hash,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		Ok(None)
	}
}
