use derive_more::Deref;

#[derive(Deref, Debug, PartialEq, Eq)]
pub struct BridgeTransferId<H>(pub H);

#[derive(Debug, PartialEq, Eq)]
pub struct BridgeTransferDetails<A, H> {
	pub bridge_transfer_id: BridgeTransferId<H>,
	pub initiator_address: A,
	pub recipient_address: A,
	pub hash_lock: H,
	pub time_lock: u64,
	pub amount: u64,
}
