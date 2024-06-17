use std::{fmt::Debug, hash::Hash};

use derive_more::Deref;

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BridgeTransferId<H>(pub H);

impl<H> GenUniqueHash for BridgeTransferId<H>
where
	H: GenUniqueHash,
{
	fn gen_unique_hash() -> Self {
		BridgeTransferId(H::gen_unique_hash())
	}
}

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InitiatorAddress<A>(pub A);

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecipientAddress<A>(pub A);

#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HashLock<H>(pub H);

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct TimeLock(pub u64);

#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct Amount(pub u64);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BridgeTransferDetails<A, H> {
	pub bridge_transfer_id: BridgeTransferId<H>,
	pub initiator_address: InitiatorAddress<A>,
	pub recipient_address: RecipientAddress<A>,
	pub hash_lock: HashLock<H>,
	pub time_lock: TimeLock,
	pub amount: Amount,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LockedAssetsDetails<A, H> {
	pub bridge_transfer_id: BridgeTransferId<H>,
	pub recipient_address: RecipientAddress<A>,
	pub hash_lock: HashLock<H>,
	pub time_lock: TimeLock,
	pub amount: Amount,
}

// Types
pub trait BridgeHashType: Debug + PartialEq + Eq + Hash + Unpin + Send + Sync + Clone {}
pub trait BridgeAddressType: Debug + PartialEq + Eq + Hash + Unpin + Send + Sync + Clone {}

// Blankets
impl<T> BridgeHashType for T where T: Debug + PartialEq + Eq + Hash + Unpin + Send + Sync + Clone {}
impl<T> BridgeAddressType for T where T: Debug + PartialEq + Eq + Hash + Unpin + Send + Sync + Clone {}

pub trait GenUniqueHash {
	fn gen_unique_hash() -> Self;
}
