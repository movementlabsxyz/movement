use delegate::delegate;
use futures::{Stream, StreamExt};
use std::{pin::Pin, sync::Arc};
use std::{
	sync::Mutex,
	task::{Context, Poll},
};

use bridge_shared::{
	blockchain_service::{BlockchainService, ContractEvent},
	bridge_contracts::BridgeContractCounterpartyResult,
	types::{HashLock, InitiatorAddress, RecipientAddress, TimeLock},
};
use bridge_shared::{
	bridge_contracts::BridgeContractInitiatorResult,
	types::{BridgeAddressType, BridgeHashType, BridgeTransferDetails, BridgeTransferId},
};
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	types::Amount,
};
use bridge_shared::{
	bridge_monitoring::{
		BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
		BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
	},
	types::HashLockPreImage,
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
	A: BridgeAddressType,
	H: BridgeHashType,
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
	A: BridgeAddressType,
	H: BridgeHashType,
{
	type Item =
		ContractEvent<<Self as BlockchainService>::Address, <Self as BlockchainService>::Hash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		if let Poll::Ready(Some(event)) = this.poll_next_event(cx) {
			return Poll::Ready(Some(event));
		}

		// poll the events from the contracts and forward them
		if let Poll::Ready(Some(event)) = this.initiator_contract.poll_next_unpin(cx) {
			return Poll::Ready(Some(ContractEvent::InitiatorEvent(event)));
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
pub struct MockInitiatorContractState<A, H> {
	events: Vec<BridgeContractInitiatorEvent<A, H>>,
	mock_next_bridge_transfer_id: Option<BridgeTransferId<H>>,
	_phantom: std::marker::PhantomData<(A, H)>,
}

impl<A, H> Default for MockInitiatorContractState<A, H> {
	fn default() -> Self {
		Self {
			events: Default::default(),
			mock_next_bridge_transfer_id: Default::default(),
			_phantom: Default::default(),
		}
	}
}

impl<A, H> MockInitiatorContractState<A, H> {
	pub fn with_next_bridge_transfer_id(&mut self, id: H) -> &mut Self {
		self.mock_next_bridge_transfer_id = Some(BridgeTransferId(id));
		self
	}
}

#[derive(Debug, Clone)]
pub struct MockInitiatorContract<A, H> {
	state: Arc<Mutex<MockInitiatorContractState<A, H>>>,
}

impl<A, H> MockInitiatorContract<A, H> {
	pub fn build() -> Self {
		Self { state: Arc::new(Mutex::new(MockInitiatorContractState::default())) }
	}

	pub fn with_next_bridge_transfer_id(&mut self, id: H) -> &mut Self {
		self.state.lock().expect("lock poisoned").with_next_bridge_transfer_id(id);
		self
	}

	delegate! {
			to self.state.lock().expect("lock poisoned") {
					// to be delegated
			}

	}
}

impl<A, H> Stream for MockInitiatorContract<A, H>
where
	A: Unpin,
	H: Unpin,
{
	type Item = BridgeContractInitiatorEvent<A, H>;

	fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		let mut state = this.state.lock().expect("lock poisoned");
		if let Some(event) = state.events.pop() {
			Poll::Ready(Some(event))
		} else {
			Poll::Pending
		}
	}
}

#[async_trait::async_trait]
impl<A, H> BridgeContractInitiator for MockInitiatorContract<A, H>
where
	A: BridgeAddressType,
	H: BridgeHashType,
{
	type Address = A;
	type Hash = H;

	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractInitiatorResult<()> {
		let mut state = self.state.lock().expect("lock poisoned");
		let next_bridge_transfer_id =
			state.mock_next_bridge_transfer_id.take().expect("no next bridge transfer id");
		state
			.events
			.push(BridgeContractInitiatorEvent::Initiated(BridgeTransferDetails {
				bridge_transfer_id: next_bridge_transfer_id,
				initiator_address,
				recipient_address,
				hash_lock,
				time_lock,
				amount,
			}));
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: HashLockPreImage,
	) -> BridgeContractInitiatorResult<()> {
		Ok(())
	}

	async fn refund_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractInitiatorResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		Ok(None)
	}
}

#[derive(Debug, Clone)]
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
	A: BridgeAddressType,
	H: BridgeHashType,
{
	type Address = A;
	type Hash = H;

	async fn lock_bridge_transfer_assets(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_hash_lock: HashLock<Self::Hash>,
		_time_lock: TimeLock,
		_recipient: RecipientAddress,
		_amount: Amount,
	) -> BridgeContractCounterpartyResult<()> {
		Ok(())
	}

	async fn complete_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: HashLockPreImage,
	) -> BridgeContractCounterpartyResult<()> {
		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractCounterpartyResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>>
	{
		Ok(None)
	}
}
