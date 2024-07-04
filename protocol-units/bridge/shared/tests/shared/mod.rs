#![allow(dead_code)] // TODO: Remove this line once the code is complete

use bridge_shared::{
	blockchain_service::AbstractBlockchainService,
	bridge_monitoring::{
		BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
		BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
	},
	bridge_service::BridgeService,
	types::{Convert, GenUniqueHash, HashLockPreImage, RecipientAddress},
};

use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use rand::Rng;
use rand::SeedableRng;
use std::{
	fmt::{Debug, Formatter},
	hash::{DefaultHasher, Hash, Hasher},
	pin::Pin,
	task::{Context, Poll},
};

pub mod testing;

use testing::{
	blockchain::AbstractBlockchainEvent,
	blockchain::{AbstractBlockchain, AbstractBlockchainClient},
	rng::{RngSeededClone, TestRng},
};

use crate::shared::testing::blockchain::{
	counterparty_contract::SmartContractCounterpartyEvent,
	initiator_contract::SmartContractInitiatorEvent,
};

pub fn hash_static_string(pre_image: &'static str) -> [u8; 8] {
	hash_vec_u8(pre_image.as_bytes())
}

pub fn hash_vec_u8(data: &[u8]) -> [u8; 8] {
	let mut hasher = DefaultHasher::new();
	data.hash(&mut hasher);
	hasher.finish().to_be_bytes()
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct BC1Hash([u8; 8]);

impl From<HashLockPreImage> for BC1Hash {
	fn from(value: HashLockPreImage) -> Self {
		Self(hash_vec_u8(&value.0))
	}
}

impl From<&'static str> for BC1Hash {
	fn from(pre_image: &'static str) -> Self {
		Self(hash_static_string(pre_image))
	}
}

impl GenUniqueHash for BC1Hash {
	fn gen_unique_hash<R: Rng>(rng: &mut R) -> Self {
		Self(rng.gen())
	}
}

impl Debug for BC1Hash {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Bc1Hash({:02x})", u64::from_be_bytes(self.0))
	}
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct BC2Hash([u8; 8]);

impl From<HashLockPreImage> for BC2Hash {
	fn from(value: HashLockPreImage) -> Self {
		Self(hash_vec_u8(&value.0))
	}
}

impl GenUniqueHash for BC2Hash {
	fn gen_unique_hash<R: Rng>(rng: &mut R) -> Self {
		Self(rng.gen())
	}
}

impl From<&'static str> for BC2Hash {
	fn from(pre_image: &'static str) -> Self {
		Self(hash_static_string(pre_image))
	}
}

impl Debug for BC2Hash {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "BC2Hash({:02x})", u64::from_be_bytes(self.0))
	}
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BC1Address(pub &'static str);

impl From<RecipientAddress> for BC1Address {
	fn from(value: RecipientAddress) -> Self {
		let string = String::from_utf8(value.0).expect("Invalid UTF-8");
		// NOTE: Using static strings in tests for clarity and efficiency. A bit of memory leakage is
		// acceptable for the rare conversions in this context.
		Self(Box::leak(string.into_boxed_str()))
	}
}

impl From<BC1Address> for RecipientAddress {
	fn from(value: BC1Address) -> Self {
		RecipientAddress(value.0.as_bytes().to_vec())
	}
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BC2Address(pub &'static str);

impl From<RecipientAddress> for BC2Address {
	fn from(value: RecipientAddress) -> Self {
		let string = String::from_utf8(value.0).expect("Invalid UTF-8");
		// NOTE: Using static strings in tests for clarity and efficiency. A bit of memory leakage is
		// acceptable for the rare conversions in this context.
		Self(Box::leak(string.into_boxed_str()))
	}
}

impl From<BC2Address> for RecipientAddress {
	fn from(value: BC2Address) -> Self {
		RecipientAddress(value.0.as_bytes().to_vec())
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

impl Convert<BC1Hash> for BC2Hash {
	fn convert(me: &BC2Hash) -> BC1Hash {
		BC1Hash(me.0)
	}
}

impl Convert<BC2Hash> for BC1Hash {
	fn convert(me: &BC1Hash) -> BC2Hash {
		BC2Hash(me.0)
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

pub struct InitiatorContractMonitoring<A, H> {
	listener: UnboundedReceiver<AbstractBlockchainEvent<A, H>>,
}

impl<A, H> InitiatorContractMonitoring<A, H> {
	pub fn build(listener: UnboundedReceiver<AbstractBlockchainEvent<A, H>>) -> Self {
		Self { listener }
	}
}

impl<A: Debug, H: Debug> BridgeContractInitiatorMonitoring for InitiatorContractMonitoring<A, H> {
	type Address = A;
	type Hash = H;
}

impl<A: Debug, H: Debug> Stream for InitiatorContractMonitoring<A, H> {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(AbstractBlockchainEvent::InitiatorContractEvent(contract_result))) =
			this.listener.poll_next_unpin(cx)
		{
			tracing::trace!(
				"InitiatorContractMonitoring: Received contract event: {:?}",
				contract_result
			);
			// Only listen to the initiator contract events
			use SmartContractInitiatorEvent::*;
			match contract_result {
				Ok(contract_event) => match contract_event {
					InitiatedBridgeTransfer(details) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Initiated(details)))
					}
					CompletedBridgeTransfer(bridge_transfer_id, _) => {
						return Poll::Ready(Some(BridgeContractInitiatorEvent::Completed(
							bridge_transfer_id,
						)))
					}
				},
				Err(_) => {
					// Handle error
				}
			}
		}
		Poll::Pending
	}
}

pub struct CounterpartyContractMonitoring<A, H> {
	listener: UnboundedReceiver<AbstractBlockchainEvent<A, H>>,
}

impl<A, H> CounterpartyContractMonitoring<A, H> {
	pub fn build(listener: UnboundedReceiver<AbstractBlockchainEvent<A, H>>) -> Self {
		Self { listener }
	}
}

impl<A: Debug, H: Debug> BridgeContractCounterpartyMonitoring
	for CounterpartyContractMonitoring<A, H>
{
	type Address = A;
	type Hash = H;
}

impl<A: Debug, H: Debug> Stream for CounterpartyContractMonitoring<A, H> {
	type Item = BridgeContractCounterpartyEvent<H>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(AbstractBlockchainEvent::CounterpartyContractEvent(
			contract_result,
		))) = this.listener.poll_next_unpin(cx)
		{
			tracing::trace!(
				"CounterpartyContractMonitoring: Received contract event: {:?}",
				contract_result
			);
			use SmartContractCounterpartyEvent::*;
			match contract_result {
				Ok(contract_event) => match contract_event {
					LockedBridgeTransfer(details) => {
						return Poll::Ready(Some(BridgeContractCounterpartyEvent::Locked(details)))
					}
					CompletedBridgeTransfer(details) => {
						return Poll::Ready(Some(BridgeContractCounterpartyEvent::Completed(
							details,
						)))
					}
				},
				Err(_) => {
					// Handle error
				}
			}
		}
		Poll::Pending
	}
}

pub type B1Client = AbstractBlockchainClient<BC1Address, BC1Hash, TestRng>;
pub type B2Client = AbstractBlockchainClient<BC2Address, BC2Hash, TestRng>;

// Setup the BlockchainService
pub type B1Service = AbstractBlockchainService<
	AbstractBlockchainClient<BC1Address, BC1Hash, TestRng>,
	InitiatorContractMonitoring<BC1Address, BC1Hash>,
	AbstractBlockchainClient<BC1Address, BC1Hash, TestRng>,
	CounterpartyContractMonitoring<BC1Address, BC1Hash>,
	BC1Address,
	BC1Hash,
>;

pub type B2Service = AbstractBlockchainService<
	AbstractBlockchainClient<BC2Address, BC2Hash, TestRng>,
	InitiatorContractMonitoring<BC2Address, BC2Hash>,
	AbstractBlockchainClient<BC2Address, BC2Hash, TestRng>,
	CounterpartyContractMonitoring<BC2Address, BC2Hash>,
	BC2Address,
	BC2Hash,
>;

pub struct SetupBridgeServiceResult(
	pub BridgeService<B1Service, B2Service>,
	pub AbstractBlockchainClient<BC1Address, BC1Hash, TestRng>,
	pub AbstractBlockchainClient<BC2Address, BC2Hash, TestRng>,
	pub AbstractBlockchain<BC1Address, BC1Hash, TestRng>,
	pub AbstractBlockchain<BC2Address, BC2Hash, TestRng>,
);

pub fn setup_bridge_service() -> SetupBridgeServiceResult {
	let mut rng = TestRng::from_seed([0u8; 32]);

	let mut blockchain_1 =
		AbstractBlockchain::<BC1Address, BC1Hash, _>::new(rng.seeded_clone(), "Blockchain1");
	let mut blockchain_2 =
		AbstractBlockchain::<BC2Address, BC2Hash, _>::new(rng.seeded_clone(), "Blockchain2");

	// Contracts and monitors for blockchain 1
	let client_1 =
		AbstractBlockchainClient::new(blockchain_1.connection(), rng.seeded_clone(), 0.0, 0.00);
	let monitor_1_initiator = InitiatorContractMonitoring::build(blockchain_1.add_event_listener());
	let monitor_1_counterparty =
		CounterpartyContractMonitoring::build(blockchain_1.add_event_listener());

	// Contracts and monitors for blockchain 2
	let client_2 =
		AbstractBlockchainClient::new(blockchain_2.connection(), rng.seeded_clone(), 0.0, 0.00);
	let monitor_2_initiator = InitiatorContractMonitoring::build(blockchain_2.add_event_listener());
	let monitor_2_counterparty =
		CounterpartyContractMonitoring::build(blockchain_2.add_event_listener());

	let blockchain_1_client = client_1.clone();
	let blockchain_1_service = AbstractBlockchainService {
		initiator_contract: blockchain_1_client.clone(),
		initiator_monitoring: monitor_1_initiator,
		counterparty_contract: blockchain_1_client.clone(),
		counterparty_monitoring: monitor_1_counterparty,
		_phantom: Default::default(),
	};

	let blockchain_2_client = client_2.clone();
	let blockchain_2_service = AbstractBlockchainService {
		initiator_contract: blockchain_2_client.clone(),
		initiator_monitoring: monitor_2_initiator,
		counterparty_contract: blockchain_2_client.clone(),
		counterparty_monitoring: monitor_2_counterparty,
		_phantom: Default::default(),
	};

	let bridge_service = BridgeService::new(blockchain_1_service, blockchain_2_service);

	SetupBridgeServiceResult(
		bridge_service,
		blockchain_1_client,
		blockchain_2_client,
		blockchain_1,
		blockchain_2,
	)
}
