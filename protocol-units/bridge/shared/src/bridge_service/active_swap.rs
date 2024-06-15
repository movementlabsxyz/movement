use std::{
	collections::HashMap,
	convert::From,
	pin::Pin,
	task::{Context, Poll},
};

use futures::{Future, FutureExt, Stream};
use thiserror::Error;
use tracing::{trace_span, Instrument};

use crate::{
	blockchain_service::BlockchainService,
	types::{BridgeTransferDetails, BridgeTransferId, HashLock},
};
use crate::{bridge_contracts::BridgeContractCounterparty, types::RecipientAddress};

pub type BoxedFuture<R, E> = Pin<Box<dyn Future<Output = Result<R, E>> + Send>>;

pub enum ActiveSwap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	LockingTokens(
		BridgeTransferDetails<BFrom::Address, BFrom::Hash>,
		BoxedFuture<(), LockBridgeTransferAssetsError>,
	),
	WaitingForCompletedEvent(BridgeTransferId<BTo::Hash>),
	CompletingBridging(BoxedFuture<(), ()>),
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

#[derive(Debug)]
pub enum ActiveSwapEvent<H> {
	BridgeAssetsLocked(BridgeTransferId<H>),
	BridgeAssetsLockingError(LockBridgeTransferAssetsError),
}

impl<BFrom, BTo> Stream for ActiveSwapMap<BFrom, BTo>
where
	BFrom: BlockchainService + Unpin + 'static,
	BTo: BlockchainService + Unpin + 'static,
{
	type Item = ActiveSwapEvent<BFrom::Hash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		for swap in this.swaps.values_mut() {
			match swap {
				ActiveSwap::LockingTokens(details, future) => {
					match future.poll_unpin(cx) {
						Poll::Ready(Ok(())) => {
							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsLocked(
								details.bridge_transfer_id.clone(),
							)));
						}
						Poll::Ready(Err(error)) => {
							// Locking tokens failed
							// Transition to the next state
							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsLockingError(
								error,
							)));
						}
						Poll::Pending => {}
					}
				}
				ActiveSwap::WaitingForCompletedEvent(_) => todo!(),
				ActiveSwap::CompletingBridging(_) => todo!(),
				ActiveSwap::Completed => todo!(),
			}
		}

		Poll::Pending
	}
}

// Lock assets

#[derive(Debug, Error)]
pub enum LockBridgeTransferAssetsError {
	#[error("Failed to lock assets")]
	LockingError,
	#[error("Failed to call lock_bridge_transfer_assets")]
	LockBridgeTransferContractCallError(String), // TODO; addd contact call errors
}

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
) -> Result<(), LockBridgeTransferAssetsError>
where
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

	Ok(())
}
