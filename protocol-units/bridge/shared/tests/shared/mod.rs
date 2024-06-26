#![allow(dead_code)] // TODO: Remove this line once the code is complete

use async_trait::async_trait;
use bridge_shared::{
	bridge_contracts::{
		BridgeContractCounterparty, BridgeContractError, BridgeContractInitiator,
		BridgeContractResult,
	},
	bridge_monitoring::{
		BridgeContractCounterpartyEvent, BridgeContractCounterpartyMonitoring,
		BridgeContractInitiatorEvent, BridgeContractInitiatorMonitoring,
	},
	types::{
		Amount, BridgeTransferDetails, BridgeTransferId, Convert, GenUniqueHash, HashLock,
		HashLockPreImage, InitiatorAddress, RecipientAddress, TimeLock,
	},
};
use futures::{channel::mpsc::UnboundedReceiver, Stream, StreamExt};
use rand::Rng;
use std::{
	fmt::Formatter,
	hash::{DefaultHasher, Hash, Hasher},
	pin::Pin,
	task::{Context, Poll},
};

pub mod testing;

use testing::{
	blockchain::{
		AbstractBlockchainClient, AbstractBlockchainEvent, CounterpartyCall, InitiatorCall,
		Transaction,
	},
	rng::TestRng,
};

use crate::shared::testing::blockchain::counterparty_contract::SmartContractCounterpartyEvent;

use self::testing::blockchain::initiator_contract::SmartContractInitiatorEvent;

pub fn hash_static_string(pre_image: &'static str) -> [u8; 8] {
	let mut hasher = DefaultHasher::new();
	pre_image.hash(&mut hasher);
	hasher.finish().to_be_bytes()
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

impl std::fmt::Debug for BC1Hash {
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

impl std::fmt::Debug for BC2Hash {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "BC2Hash({:02x})", u64::from_be_bytes(self.0))
	}
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BC1Address(pub &'static str);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BC2Address(pub &'static str);

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

impl<A, H> BridgeContractInitiatorMonitoring for InitiatorContractMonitoring<A, H> {
	type Address = A;
	type Hash = H;
}

impl<A, H> Stream for InitiatorContractMonitoring<A, H> {
	type Item = BridgeContractInitiatorEvent<
		<Self as BridgeContractInitiatorMonitoring>::Address,
		<Self as BridgeContractInitiatorMonitoring>::Hash,
	>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(event)) = this.listener.poll_next_unpin(cx) {
			// Only listen to the initiator contract events
			if let AbstractBlockchainEvent::InitiatorContractEvent(contract_result) = event {
				use SmartContractInitiatorEvent::*;
				match contract_result {
					Ok(contract_event) => match contract_event {
						InitiatedBridgeTransfer(details) => {
							return Poll::Ready(Some(BridgeContractInitiatorEvent::Initiated(
								details,
							)))
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

impl<A, H> BridgeContractCounterpartyMonitoring for CounterpartyContractMonitoring<A, H> {
	type Address = A;
	type Hash = H;
}

impl<A, H> Stream for CounterpartyContractMonitoring<A, H> {
	type Item = BridgeContractCounterpartyEvent<A, H>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		if let Poll::Ready(Some(event)) = this.listener.poll_next_unpin(cx) {
			if let AbstractBlockchainEvent::CounterpartyContractEvent(contract_result) = event {
				use SmartContractCounterpartyEvent::*;
				match contract_result {
					Ok(contract_event) => match contract_event {
						LockedBridgeTransfer(details) => {
							return Poll::Ready(Some(BridgeContractCounterpartyEvent::Locked(
								details,
							)))
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
		}
		Poll::Pending
	}
}

#[derive(Clone)]
pub struct B1Client {
	client: AbstractBlockchainClient<BC1Address, BC1Hash, TestRng>,
}

impl B1Client {
	pub fn build(client: AbstractBlockchainClient<BC1Address, BC1Hash, TestRng>) -> Self {
		Self { client }
	}
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
		self.client.send_transaction(transaction).map_err(BridgeContractError::generic)
	}

	async fn complete_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: HashLockPreImage,
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

	async fn complete_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: HashLockPreImage,
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
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
	) -> BridgeContractResult<Option<BridgeTransferDetails<Self::Hash, Self::Address>>> {
		Ok(None)
	}
}

#[derive(Clone)]
pub struct B2Client {
	client: AbstractBlockchainClient<BC2Address, BC2Hash, TestRng>,
}

impl B2Client {
	pub fn build(client: AbstractBlockchainClient<BC2Address, BC2Hash, TestRng>) -> Self {
		Self { client }
	}
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
		self.client.send_transaction(transaction).map_err(BridgeContractError::generic)
	}

	async fn complete_bridge_transfer(
		&mut self,
		bridge_transfer_id: BridgeTransferId<Self::Hash>,
		secret: HashLockPreImage,
	) -> BridgeContractResult<()> {
		let transaction = Transaction::Initiator(InitiatorCall::CompleteBridgeTransfer(
			bridge_transfer_id,
			secret,
		));
		self.client.send_transaction(transaction).map_err(BridgeContractError::generic)
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

	async fn complete_bridge_transfer(
		&mut self,
		_bridge_transfer_id: BridgeTransferId<Self::Hash>,
		_secret: HashLockPreImage,
	) -> BridgeContractResult<()> {
		let transaction = Transaction::Counterparty(CounterpartyCall::CompleteBridgeTransfer(
			_bridge_transfer_id,
			_secret,
		));
		self.client.send_transaction(transaction).map_err(BridgeContractError::generic)
	}

	async fn abort_bridge_transfer(
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
