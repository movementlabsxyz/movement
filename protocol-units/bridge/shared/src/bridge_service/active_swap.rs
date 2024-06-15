use std::{collections::HashMap, convert::From, pin::Pin};

use futures::{Future, FutureExt};
use tracing::{trace_span, Instrument};

use crate::{
	blockchain_service::BlockchainService,
	types::{BridgeTransferDetails, BridgeTransferId, HashLock},
};
use crate::{bridge_contracts::BridgeContractCounterparty, types::RecipientAddress};

pub type BoxedBridgeServiceFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Bridge state in the sense of tracking the active swaps from the bridge
pub enum ActiveSwap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	/// Bridge is locking lockens on the counterpart chain
	LockingTokens(BridgeTransferDetails<BFrom::Address, BFrom::Hash>, BoxedBridgeServiceFuture),
	/// Bridge is waiting for the initiator to complete her transfer
	/// revealing her secret.
	WaitingForCompletedEvent(BridgeTransferId<BTo::Hash>),
	/// We are in possession of the secret and are now completing the bridge tranfer on our side
	CompletingBridging(BoxedBridgeServiceFuture),
	/// We have completed the atomic bdridge transfer
	Completed,
}

pub struct ActiveSwapMap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	pub initiator_contract: BFrom::InitiatorContract,
	pub counterparty_contract: BTo::CounterpartyContract,
	swaps: HashMap<BridgeTransferId<BFrom::Hash>, ActiveSwap<BFrom, BTo>>,
}

impl<BTo, BFrom> ActiveSwapMap<BFrom, BTo>
where
	BTo: BlockchainService + 'static,
	BFrom: BlockchainService + 'static,
{
	pub fn build(
		initiator_contract: BFrom::InitiatorContract,
		counterparty_contract: BTo::CounterpartyContract,
	) -> Self {
		Self { initiator_contract, counterparty_contract, swaps: HashMap::new() }
	}

	pub fn get(&self, key: &BridgeTransferId<BFrom::Hash>) -> Option<&ActiveSwap<BFrom, BTo>> {
		self.swaps.get(key)
	}

	pub fn already_executing(&self, key: &BridgeTransferId<BFrom::Hash>) -> bool {
		self.swaps.contains_key(key)
	}

	pub fn start(&mut self, details: BridgeTransferDetails<BFrom::Address, BFrom::Hash>)
	where
		<BTo::CounterpartyContract as BridgeContractCounterparty>::Hash: From<BFrom::Hash>,
		<BTo::CounterpartyContract as BridgeContractCounterparty>::Address: From<BFrom::Address>,
	{
		assert!(self.swaps.get(&details.bridge_transfer_id).is_none());

		let counterparty_contract = self.counterparty_contract.clone();
		let bridge_transfer_id = details.bridge_transfer_id.clone();

		tracing::trace!("Starting active swap for bridge transfer {:?}", bridge_transfer_id);

		self.swaps.insert(
			bridge_transfer_id,
			ActiveSwap::<BFrom, BTo>::LockingTokens(
				details.clone(),
				call_lock_bridge_transfer_assets::<BFrom, BTo>(counterparty_contract, details)
					.instrument(trace_span!(
						"call_lock_bridge_transfer_assets[{bridge_transfer_id:?}]"
					))
					.boxed(),
			),
		);
	}
}

/// Making sure we call the method using the correct details on the counterparty contract
async fn call_lock_bridge_transfer_assets<BFrom: BlockchainService, BTo: BlockchainService>(
	counterparty_contract: BTo::CounterpartyContract,
	BridgeTransferDetails {
		bridge_transfer_id,
		hash_lock,
		time_lock,
		recipient_address,
		amount,
		..
	}: BridgeTransferDetails<BFrom::Address, BFrom::Hash>,
) where
	<BTo::CounterpartyContract as BridgeContractCounterparty>::Hash: From<BFrom::Hash>,
	<BTo::CounterpartyContract as BridgeContractCounterparty>::Address: From<BFrom::Address>,
{
	let bridge_transfer_id = BridgeTransferId(From::from(bridge_transfer_id.0));
	let hash_lock = HashLock(From::from(hash_lock.0));
	let recipient_address = RecipientAddress(From::from(recipient_address.0));

	tracing::trace!(
		"Calling lock_bridge_transfer_assets on counterparty contract for bridge transfer {:?}",
		bridge_transfer_id
	);

	counterparty_contract
		.lock_bridge_transfer_assets(
			bridge_transfer_id,
			hash_lock,
			time_lock,
			recipient_address,
			amount,
		)
		.await;
}
