use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

use bridge_shared::bridge_monitoring::{
	BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
	BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
};
use bridge_shared::types::{BridgeTransferDetails, BridgeTransferId};
use bridge_shared::{
	blockchain_service::{BlockchainEvent, BlockchainService},
	types::{HashLock, InitiatorAddress, RecipientAddress, TimeLock},
};
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator, BridgeContractResult},
	types::Amount,
};

struct MockBlockchainService {
	initiator_contract: MockInitiatorContract,
	initiator_monitoring: MockInitiatorMonitoring,
	counterparty_contract: MockCounterpartyContract,
	counterparty_monitoring: MockCounterpartyMonitoring,
}

impl BlockchainService for MockBlockchainService {
	type Address = &'static str;
	type Hash = &'static str;

	type InitiatorContract = MockInitiatorContract;
	type InitiatorMonitoring = MockInitiatorMonitoring;

	type CounterpartyContract = MockCounterpartyContract;
	type CounterpartyMonitoring = MockCounterpartyMonitoring;

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

impl Stream for MockBlockchainService {
	type Item =
		BlockchainEvent<<Self as BlockchainService>::Address, <Self as BlockchainService>::Hash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.poll_next_event(cx)
	}
}

struct MockInitiatorMonitoring {
	events: Vec<
		BridgeContractInitiatorEvent<
			<Self as BridgeContractInitiatorMonitoring>::Address,
			<Self as BridgeContractInitiatorMonitoring>::Hash,
		>,
	>,
}

impl BridgeContractInitiatorMonitoring for MockInitiatorMonitoring {
	type Address = &'static str;
	type Hash = &'static str;
}

impl Stream for MockInitiatorMonitoring {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Some(event) = this.events.pop() {
			Poll::Ready(Some(event))
		} else {
			Poll::Pending
		}
	}
}

struct MockCounterpartyMonitoring {
	events: Vec<
		BridgeContractCounterpartyEvent<
			<Self as BridgeContractCounterpartyMonitoring>::Address,
			<Self as BridgeContractCounterpartyMonitoring>::Hash,
		>,
	>,
}

impl BridgeContractCounterpartyMonitoring for MockCounterpartyMonitoring {
	type Address = &'static str;
	type Hash = &'static str;
}

impl Stream for MockCounterpartyMonitoring {
	type Item = BridgeContractCounterpartyEvent<
		<Self as BridgeContractCounterpartyMonitoring>::Address,
		<Self as BridgeContractCounterpartyMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Some(event) = this.events.pop() {
			Poll::Ready(Some(event))
		} else {
			Poll::Pending
		}
	}
}

struct MockInitiatorContract;

#[async_trait::async_trait]
impl BridgeContractInitiator for MockInitiatorContract {
	type Address = &'static str;
	type Hash = &'static str;

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
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>> {
		Ok(None)
	}
}

struct MockCounterpartyContract;

#[async_trait::async_trait]
impl BridgeContractCounterparty for MockCounterpartyContract {
	type Address = &'static str;
	type Hash = &'static str;

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
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Address, Self::Hash>>> {
		Ok(None)
	}
}

#[tokio::test]
async fn test_bridge_transfer_initiated() {
	let initiator_monitoring = MockInitiatorMonitoring {
		events: vec![BridgeContractInitiatorEvent::BridgeTransferInitiated(
			BridgeTransferDetails {
				bridge_transfer_id: BridgeTransferId("transfer_id"),
				initiator_address: InitiatorAddress("initiator"),
				recipient_address: RecipientAddress("recipient"),
				hash_lock: HashLock("hash_lock"),
				time_lock: TimeLock(100),
				amount: Amount(1000),
			},
		)],
	};

	let counterparty_monitoring = MockCounterpartyMonitoring { events: vec![] };

	let initiator_contract = MockInitiatorContract;
	let counterparty_contract = MockCounterpartyContract;

	let mut blockchain_service = MockBlockchainService {
		initiator_contract,
		initiator_monitoring,
		counterparty_contract,
		counterparty_monitoring,
	};

	let mut cx = Context::from_waker(futures::task::noop_waker_ref());
	let event = blockchain_service.poll_next_unpin(&mut cx);

	assert_eq!(
		event,
		Poll::Ready(Some(BlockchainEvent::InitiatorEvent(
			BridgeContractInitiatorEvent::BridgeTransferInitiated(BridgeTransferDetails {
				bridge_transfer_id: BridgeTransferId("transfer_id"),
				initiator_address: InitiatorAddress("initiator"),
				recipient_address: RecipientAddress("recipient"),
				hash_lock: HashLock("hash_lock"),
				time_lock: TimeLock(100),
				amount: Amount(1000),
			})
		)))
	);
}
