use futures::Stream;
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

impl<A, H> BlockchainService for MockBlockchainService<A, H>
where
	A: std::fmt::Debug + Unpin + Send + Sync,
	H: std::fmt::Debug + Unpin + Send + Sync,
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
	H: std::fmt::Debug + Unpin + Send + Sync,
{
	type Item =
		BlockchainEvent<<Self as BlockchainService>::Address, <Self as BlockchainService>::Hash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.poll_next_event(cx)
	}
}

pub struct MockInitiatorMonitoring<A, H> {
	pub events: Vec<BridgeContractInitiatorEvent<A, H>>,
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

pub struct MockInitiatorContract<A, H> {
	_phantom: std::marker::PhantomData<(A, H)>,
}

impl<A, H> MockInitiatorContract<A, H> {
	pub fn build() -> Self {
		Self { _phantom: std::marker::PhantomData }
	}
}

#[async_trait::async_trait]
impl<A, H> BridgeContractInitiator for MockInitiatorContract<A, H>
where
	A: std::fmt::Debug + Send + Sync,
	H: std::fmt::Debug + Send + Sync,
{
	type Address = A;
	type Hash = H;

	async fn initiate_bridge_transfer(
		&self,
		_initiator_address: InitiatorAddress<Self::Address>,
		_recipient_address: RecipientAddress<Self::Address>,
		_hash_lock: HashLock<Self::Hash>,
		_time_lock: TimeLock,
		_amount: Amount,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn complete_bridge_transfer<S: Send>(
		&self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: S,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn refund_bridge_transfer(
		&self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&self,
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
