use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use rand::SeedableRng;
use std::{
	pin::Pin,
	task::{Context, Poll},
};
use test_log::test;

use async_trait::async_trait;
use bridge_shared::{
	blockchain_service::BlockchainEvent,
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, HashLock, InitiatorAddress,
		RecipientAddress, TimeLock,
	},
};
use bridge_shared::{blockchain_service::BlockchainService, testing::rng::TestRng};
use bridge_shared::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractResult},
	types::GenUniqueHash,
};
use bridge_shared::{
	bridge_contracts::{BridgeContractError, BridgeContractInitiator},
	bridge_monitoring::BridgeContractInitiatorMonitoring,
};
use bridge_shared::{
	bridge_monitoring::BridgeContractCounterpartyEvent,
	testing::blockchain::{CounterpartyCall, InitiatorCall, Transaction},
};
use bridge_shared::{
	bridge_monitoring::BridgeContractCounterpartyMonitoring,
	testing::blockchain::{AbstractBlockchain, AbstractBlockchainClient, AbstractBlockchainEvent},
};
use bridge_shared::{
	bridge_monitoring::BridgeContractInitiatorEvent, bridge_service::BridgeService,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BC1Hash(pub &'static str);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BC2Hash(pub &'static str);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BC1Address(pub &'static str);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BC2Address(pub &'static str);

impl GenUniqueHash for BC1Hash {
	fn gen_unique_hash() -> Self {
		Self("unique_hash")
	}
}

impl GenUniqueHash for BC2Hash {
	fn gen_unique_hash() -> Self {
		Self("unique_hash")
	}
}

impl From<BC1Address> for BC2Address {
	fn from(address: BC1Address) -> Self {
		Self(address.0)
	}
}

impl From<BC2Address> for BC1Address {
	fn from(address: BC2Address) -> Self {
		Self(address.0)
	}
}

impl From<BC1Hash> for BC2Hash {
	fn from(hash: BC1Hash) -> Self {
		Self(hash.0)
	}
}

impl From<BC2Hash> for BC1Hash {
	fn from(hash: BC2Hash) -> Self {
		Self(hash.0)
	}
}

pub struct B1InitiatorContractMonitoring {
	listener: UnboundedReceiver<AbstractBlockchainEvent<BC1Address, BC1Hash>>,
}

impl BridgeContractInitiatorMonitoring for B1InitiatorContractMonitoring {
	type Address = BC1Address;
	type Hash = BC1Hash;
}

impl Stream for B1InitiatorContractMonitoring {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(event)) = this.listener.poll_next_unpin(cx) {
			match event {
				AbstractBlockchainEvent::BridgeTransferInitiated(details) => {
					return Poll::Ready(Some(
						BridgeContractInitiatorEvent::BridgeTransferInitiated(details),
					))
				}
				_ => return Poll::Pending,
			}
		}
		Poll::Pending
	}
}

pub struct B2InitiatorContractMonitoring {
	listener: UnboundedReceiver<AbstractBlockchainEvent<BC2Address, BC2Hash>>,
}
impl BridgeContractInitiatorMonitoring for B2InitiatorContractMonitoring {
	type Address = BC2Address;
	type Hash = BC2Hash;
}

impl Stream for B2InitiatorContractMonitoring {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(event)) = this.listener.poll_next_unpin(cx) {
			match event {
				AbstractBlockchainEvent::BridgeTransferInitiated(details) => {
					return Poll::Ready(Some(
						BridgeContractInitiatorEvent::BridgeTransferInitiated(details),
					))
				}
				_ => return Poll::Pending,
			}
		}
		Poll::Pending
	}
}

pub struct B1CounterpartyContractMonitoring {
	listener: UnboundedReceiver<AbstractBlockchainEvent<BC1Address, BC1Hash>>,
}

impl BridgeContractCounterpartyMonitoring for B1CounterpartyContractMonitoring {
	type Address = BC1Address;
	type Hash = BC1Hash;
}

impl Stream for B1CounterpartyContractMonitoring {
	type Item = BridgeContractCounterpartyEvent<
		<Self as BridgeContractCounterpartyMonitoring>::Address,
		<Self as BridgeContractCounterpartyMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(event)) = this.listener.poll_next_unpin(cx) {
			match event {
				AbstractBlockchainEvent::BridgeTransferAssetsLocked(details) => {
					return Poll::Ready(Some(
						BridgeContractCounterpartyEvent::BridgeTransferLocked(details),
					))
				}
				_ => return Poll::Pending,
			}
		}
		Poll::Pending
	}
}

pub struct B2CounterpartyContractMonitoring {
	listener: UnboundedReceiver<AbstractBlockchainEvent<BC2Address, BC2Hash>>,
}

impl BridgeContractCounterpartyMonitoring for B2CounterpartyContractMonitoring {
	type Address = BC2Address;
	type Hash = BC2Hash;
}

impl Stream for B2CounterpartyContractMonitoring {
	type Item = BridgeContractCounterpartyEvent<
		<Self as BridgeContractCounterpartyMonitoring>::Address,
		<Self as BridgeContractCounterpartyMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(event)) = this.listener.poll_next_unpin(cx) {
			match event {
				AbstractBlockchainEvent::BridgeTransferAssetsLocked(details) => {
					return Poll::Ready(Some(
						BridgeContractCounterpartyEvent::BridgeTransferLocked(details),
					))
				}
				_ => return Poll::Pending,
			}
		}
		Poll::Pending
	}
}

#[derive(Clone)]
pub struct B1Client {
	client: AbstractBlockchainClient<BC1Address, BC1Hash, TestRng>,
}

#[async_trait]
impl BridgeContractInitiator for B1Client {
	type Address = BC1Address;
	type Hash = BC1Hash;

	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Self::Address>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractResult<()> {
		let transaction = Transaction::Initiator(InitiatorCall::InitiateBridgeTransfer(
			initiator_address,
			recipient_address,
			amount,
			time_lock,
			hash_lock,
		));
		self.client
			.send_transaction(transaction)
			.map_err(BridgeContractError::GenericError)
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

#[async_trait]
impl BridgeContractCounterparty for B1Client {
	type Address = BC1Address;
	type Hash = BC1Hash;

	async fn lock_bridge_transfer_assets(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		recipient: RecipientAddress<Self::Address>,
		amount: Amount,
	) -> bool {
		let transaction = Transaction::Counterparty(CounterpartyCall::LockBridgeTransfer(
			bridge_transfer_id,
			hash_lock,
			time_lock,
			recipient,
			amount,
		));
		self.client.send_transaction(transaction).is_ok()
	}

	async fn complete_bridge_transfer<S: Send>(
		&mut self,
		_bridge_transfer_id: Self::Hash,
		_secret: S,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: Self::Hash,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		Ok(None)
	}
}

#[derive(Clone)]
pub struct B2Client {
	client: AbstractBlockchainClient<BC2Address, BC2Hash, TestRng>,
}

#[async_trait]
impl BridgeContractInitiator for B2Client {
	type Address = BC2Address;
	type Hash = BC2Hash;

	async fn initiate_bridge_transfer(
		&mut self,
		initiator_address: InitiatorAddress<Self::Address>,
		recipient_address: RecipientAddress<Self::Address>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		amount: Amount,
	) -> BridgeContractResult<()> {
		let transaction = Transaction::Initiator(InitiatorCall::InitiateBridgeTransfer(
			initiator_address,
			recipient_address,
			amount,
			time_lock,
			hash_lock,
		));
		self.client
			.send_transaction(transaction)
			.map_err(BridgeContractError::GenericError)
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

#[async_trait]
impl BridgeContractCounterparty for B2Client {
	type Address = BC2Address;
	type Hash = BC2Hash;

	async fn lock_bridge_transfer_assets(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		hash_lock: HashLock<Self::Hash>,
		time_lock: TimeLock,
		recipient: RecipientAddress<Self::Address>,
		amount: Amount,
	) -> bool {
		let transaction = Transaction::Counterparty(CounterpartyCall::LockBridgeTransfer(
			bridge_transfer_id,
			hash_lock,
			time_lock,
			recipient,
			amount,
		));
		self.client.send_transaction(transaction).is_ok()
	}

	async fn complete_bridge_transfer<S: Send>(
		&mut self,
		_bridge_transfer_id: Self::Hash,
		_secret: S,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn abort_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<()> {
		Ok(())
	}

	async fn get_bridge_transfer_details(
		&mut self,
		_bridge_transfer_id: Self::Hash,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		Ok(None)
	}
}

#[test(tokio::test)]
async fn test_bridge_service_integration() {
	let rng1 = TestRng::from_seed([0u8; 32]);
	let rng2 = TestRng::from_seed([1u8; 32]);

	let mut blockchain_1 = AbstractBlockchain::<BC1Address, BC1Hash, _>::new(rng1, "Blockchain1");
	let mut blockchain_2 = AbstractBlockchain::<BC2Address, BC2Hash, _>::new(rng2, "Blockchain2");

	let client_1 = AbstractBlockchainClient::new(
		blockchain_1.connection(),
		TestRng::from_seed([0u8; 32]),
		0.1,
		0.05,
	);

	let monitor_1_initiator =
		B1InitiatorContractMonitoring { listener: blockchain_1.add_event_listener() };
	let monitor_1_counterparty =
		B1CounterpartyContractMonitoring { listener: blockchain_1.add_event_listener() };

	let client_2 = AbstractBlockchainClient::new(
		blockchain_2.connection(),
		TestRng::from_seed([1u8; 32]),
		0.1,
		0.05,
	);

	let monitor_2_initiator =
		B2InitiatorContractMonitoring { listener: blockchain_2.add_event_listener() };

	let monitor_2_counterparty =
		B2CounterpartyContractMonitoring { listener: blockchain_2.add_event_listener() };

	pub struct B1Service {
		pub initiator_contract: B1Client,
		pub initiator_monitoring: B1InitiatorContractMonitoring,
		pub counterparty_contract: B1Client,
		pub counterparty_monitoring: B1CounterpartyContractMonitoring,
	}

	impl BlockchainService for B1Service {
		type Address = BC1Address;
		type Hash = BC1Hash;

		type InitiatorContract = B1Client;
		type CounterpartyContract = B1Client;
		type InitiatorMonitoring = B1InitiatorContractMonitoring;
		type CounterpartyMonitoring = B1CounterpartyContractMonitoring;

		fn initiator_contract(&self) -> &Self::InitiatorContract {
			&self.initiator_contract
		}

		fn counterparty_contract(&self) -> &Self::CounterpartyContract {
			&self.counterparty_contract
		}

		fn initiator_monitoring(&mut self) -> &mut Self::InitiatorMonitoring {
			&mut self.initiator_monitoring
		}

		fn counterparty_monitoring(&mut self) -> &mut Self::CounterpartyMonitoring {
			&mut self.counterparty_monitoring
		}
	}

	impl Stream for B1Service {
		type Item = BlockchainEvent<
			<Self as BlockchainService>::Address,
			<Self as BlockchainService>::Hash,
		>;

		fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
			let this = self.get_mut();
			this.poll_next_event(cx)
		}
	}

	let blockchain_service_1 = B1Service {
		initiator_contract: B1Client { client: client_1.clone() },
		initiator_monitoring: monitor_1_initiator,
		counterparty_contract: B1Client { client: client_1 },
		counterparty_monitoring: monitor_1_counterparty,
	};
	pub struct B2Service {
		pub initiator_contract: B2Client,
		pub initiator_monitoring: B2InitiatorContractMonitoring,
		pub counterparty_contract: B2Client,
		pub counterparty_monitoring: B2CounterpartyContractMonitoring,
	}

	impl BlockchainService for B2Service {
		type Address = BC2Address;
		type Hash = BC2Hash;

		type InitiatorContract = B2Client;
		type CounterpartyContract = B2Client;
		type InitiatorMonitoring = B2InitiatorContractMonitoring;
		type CounterpartyMonitoring = B2CounterpartyContractMonitoring;

		fn initiator_contract(&self) -> &Self::InitiatorContract {
			&self.initiator_contract
		}

		fn counterparty_contract(&self) -> &Self::CounterpartyContract {
			&self.counterparty_contract
		}

		fn initiator_monitoring(&mut self) -> &mut Self::InitiatorMonitoring {
			&mut self.initiator_monitoring
		}

		fn counterparty_monitoring(&mut self) -> &mut Self::CounterpartyMonitoring {
			&mut self.counterparty_monitoring
		}
	}

	impl Stream for B2Service {
		type Item = BlockchainEvent<
			<Self as BlockchainService>::Address,
			<Self as BlockchainService>::Hash,
		>;

		fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
			let this = self.get_mut();
			this.poll_next_event(cx)
		}
	}

	let blockchain_service_2 = B2Service {
		initiator_contract: B2Client { client: client_2.clone() },
		initiator_monitoring: monitor_2_initiator,
		counterparty_contract: B2Client { client: client_2 },
		counterparty_monitoring: monitor_2_counterparty,
	};

	let mut bridge_service = BridgeService::new(blockchain_service_1, blockchain_service_2);

	let mut cx = Context::from_waker(futures::task::noop_waker_ref());
	let _ = bridge_service.poll_next_unpin(&mut cx);
	let _ = bridge_service.poll_next_unpin(&mut cx);
	let _ = bridge_service.poll_next_unpin(&mut cx);
}
