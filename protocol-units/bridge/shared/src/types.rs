use derive_more::Deref;

#[derive(Deref, Debug, PartialEq, Eq)]
pub struct BridgeTransferId<H>(pub H);

#[derive(Deref, Debug, PartialEq, Eq)]
pub struct InitiatorAddress<A>(pub A);

#[derive(Deref, Debug, PartialEq, Eq)]
pub struct RecipientAddress<A>(pub A);

#[derive(Deref, Debug, PartialEq, Eq)]
pub struct HashLock<H>(pub H);

#[derive(Deref, Debug, PartialEq, Eq)]
pub struct TimeLock(pub u64);

#[derive(Deref, Debug, PartialEq, Eq)]
pub struct Amount(pub u64);

#[derive(Debug, PartialEq, Eq)]
pub struct BridgeTransferDetails<A, H> {
	pub bridge_transfer_id: BridgeTransferId<H>,
	pub initiator_address: InitiatorAddress<A>,
	pub recipient_address: RecipientAddress<A>,
	pub hash_lock: HashLock<H>,
	pub time_lock: TimeLock,
	pub amount: Amount,
}
