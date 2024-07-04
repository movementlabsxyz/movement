use derive_more::{Deref, DerefMut};
use hex::FromHexError;
use rand::Rng;
use std::{fmt, fmt::Debug, hash::Hash};

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BridgeTransferId<H>(pub H);

impl BridgeTransferId<[u8; 32]> {
	pub fn parse(s: &str) -> Result<Self, FromHexError> {
		let bytes = hex::decode(s)?;
		let array: [u8; 32] =
			bytes.as_slice().try_into().map_err(|_| FromHexError::InvalidStringLength)?;
		Ok(BridgeTransferId(array))
	}
}

impl fmt::Display for BridgeTransferId<[u8; 32]> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", hex::encode(self.0))
	}
}

impl<H, O> Convert<BridgeTransferId<O>> for BridgeTransferId<H>
where
	H: Convert<O>,
{
	fn convert(me: &BridgeTransferId<H>) -> BridgeTransferId<O> {
		BridgeTransferId(Convert::convert(&me.0))
	}
}

impl<H> From<H> for BridgeTransferId<H> {
	fn from(hash: H) -> Self {
		BridgeTransferId(hash)
	}
}

pub fn convert_bridge_transfer_id<H: From<O>, O>(
	other: BridgeTransferId<O>,
) -> BridgeTransferId<H> {
	BridgeTransferId(From::from(other.0))
}

impl<H> GenUniqueHash for BridgeTransferId<H>
where
	H: GenUniqueHash,
{
	fn gen_unique_hash<R: Rng>(rng: &mut R) -> Self {
		BridgeTransferId(H::gen_unique_hash(rng))
	}
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InitiatorAddress<A>(pub A);

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecipientAddress(pub Vec<u8>);

impl From<&str> for RecipientAddress {
	fn from(value: &str) -> Self {
		RecipientAddress(value.as_bytes().to_vec())
	}
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HashLock<H>(pub H);

impl HashLock<[u8; 32]> {
	pub fn parse(s: &str) -> Result<Self, FromHexError> {
		let bytes = hex::decode(s)?;
		let array: [u8; 32] =
			bytes.as_slice().try_into().map_err(|_| FromHexError::InvalidStringLength)?;
		Ok(HashLock(array))
	}
}

pub fn convert_hash_lock<H: From<O>, O>(other: HashLock<O>) -> HashLock<H> {
	HashLock(From::from(other.0))
}

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct HashLockPreImage(pub Vec<u8>);

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct TimeLock(pub u64);

#[derive(Deref, DerefMut, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount(pub u64);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BridgeTransferState {
	Initialized,
	Completed,
	Refunded,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BridgeTransferDetails<A, H> {
	pub bridge_transfer_id: BridgeTransferId<H>,
	pub initiator_address: InitiatorAddress<A>,
	pub recipient_address: RecipientAddress,
	pub hash_lock: HashLock<H>,
	pub time_lock: TimeLock,
	pub amount: Amount,
	pub state: BridgeTransferState,
}

/// Defailt for testing purposes only
impl Default for BridgeTransferDetails<RecipientAddress, [u8; 32]> {
	fn default() -> Self {
		BridgeTransferDetails {
			bridge_transfer_id: BridgeTransferId([0; 32]),
			initiator_address: InitiatorAddress(RecipientAddress(vec![])),
			recipient_address: RecipientAddress(vec![]),
			hash_lock: HashLock([0; 32]),
			time_lock: TimeLock(0),
			amount: Amount(0),
			state: BridgeTransferState::Initialized,
		}
	}
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LockDetails<A, H> {
	pub bridge_transfer_id: BridgeTransferId<H>,
	pub recipient_address: RecipientAddress,
	pub hash_lock: HashLock<H>,
	pub time_lock: TimeLock,
	pub amount: Amount,
	pub _phantom: std::marker::PhantomData<A>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CompletedDetails<A, H> {
	pub bridge_transfer_id: BridgeTransferId<H>,
	pub recipient_address: RecipientAddress,
	pub hash_lock: HashLock<H>,
	pub secret: HashLockPreImage,
	pub amount: Amount,
	pub _phantom: std::marker::PhantomData<A>,
}

impl<A, H> CompletedDetails<A, H> {
	pub fn from_bridge_transfer_details(
		bridge_transfer_details: BridgeTransferDetails<A, H>,
		secret: HashLockPreImage,
	) -> Self {
		CompletedDetails {
			bridge_transfer_id: bridge_transfer_details.bridge_transfer_id,
			recipient_address: bridge_transfer_details.recipient_address,
			hash_lock: bridge_transfer_details.hash_lock,
			secret,
			amount: bridge_transfer_details.amount,
			_phantom: std::marker::PhantomData,
		}
	}

	pub fn from_lock_details(lock_details: LockDetails<A, H>, secret: HashLockPreImage) -> Self {
		CompletedDetails {
			bridge_transfer_id: lock_details.bridge_transfer_id,
			recipient_address: lock_details.recipient_address,
			hash_lock: lock_details.hash_lock,
			secret,
			amount: lock_details.amount,
			_phantom: std::marker::PhantomData,
		}
	}
}

// Types
pub trait BridgeHashType: Debug + PartialEq + Eq + Hash + Unpin + Send + Sync + Clone {}
pub trait BridgeAddressType: Debug + PartialEq + Eq + Hash + Unpin + Send + Sync + Clone {}

pub trait Convert<O> {
	fn convert(other: &Self) -> O;
}

// Blankets
impl<T> BridgeHashType for T where T: Debug + PartialEq + Eq + Hash + Unpin + Send + Sync + Clone {}
impl<T> BridgeAddressType for T where T: Debug + PartialEq + Eq + Hash + Unpin + Send + Sync + Clone {}

pub trait GenUniqueHash {
	fn gen_unique_hash<R: Rng>(rng: &mut R) -> Self;
}
