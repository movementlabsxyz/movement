use std::{
	collections::HashMap,
	convert::From,
	pin::Pin,
	task::{Context, Poll},
};

use futures::{Future, FutureExt, Stream};
use futures_timer::Delay;
use thiserror::Error;

use crate::{
	blockchain_service::BlockchainService,
	types::{BridgeTransferDetails, BridgeTransferId, HashLock},
};
use crate::{bridge_contracts::BridgeContractCounterparty, types::RecipientAddress};

pub type BoxedFuture<R, E> = Pin<Box<dyn Future<Output = Result<R, E>> + Send>>;

struct ActiveSwap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	pub details: BridgeTransferDetails<BFrom::Address, BFrom::Hash>,
	pub state: ActiveSwapState,
	_phantom: std::marker::PhantomData<BTo>,
}

pub enum ActiveSwapState {
	LockingTokens(BoxedFuture<(), LockBridgeTransferAssetsError>),
	LockingTokensError(usize, Delay),
	WaitingForCompletedEvent,
	CompletingBridging(BoxedFuture<(), ()>),
	Completed,
}

pub struct ActiveSwapConfig {
	error_attempts: usize,
	error_delay: std::time::Duration,
}
impl Default for ActiveSwapConfig {
	fn default() -> Self {
		Self { error_attempts: 3, error_delay: std::time::Duration::from_secs(5) }
	}
}

pub struct ActiveSwapMap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	pub config: ActiveSwapConfig,
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
		Self {
			initiator_contract,
			counterparty_contract,
			swaps: HashMap::new(),
			config: ActiveSwapConfig::default(),
		}
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
			ActiveSwap {
				details: details.clone(),
				state: ActiveSwapState::LockingTokens(
					call_lock_bridge_transfer_assets::<BFrom, BTo>(counterparty_contract, details)
						.boxed(),
				),
				_phantom: std::marker::PhantomData,
			},
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
	<BTo::CounterpartyContract as BridgeContractCounterparty>::Hash: From<BFrom::Hash>,
	<BTo::CounterpartyContract as BridgeContractCounterparty>::Address: From<BFrom::Address>,
{
	type Item = ActiveSwapEvent<BFrom::Hash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		for ActiveSwap { details, state, .. } in this.swaps.values_mut() {
			match state {
				ActiveSwapState::LockingTokens(future) => {
					match future.poll_unpin(cx) {
						Poll::Ready(Ok(())) => {
							*state = ActiveSwapState::WaitingForCompletedEvent;

							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsLocked(
								details.bridge_transfer_id.clone(),
							)));
						}
						Poll::Ready(Err(error)) => {
							// Locking tokens failed
							// Transition to the next state
							*state = ActiveSwapState::LockingTokensError(
								this.config.error_attempts,
								Delay::new(this.config.error_delay),
							);
							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsLockingError(
								error,
							)));
						}
						Poll::Pending => {}
					}
				}
				ActiveSwapState::LockingTokensError(attempt, delay) => {
					// test if the delay has expired
					// if it has, retry the lock
					if let Poll::Ready(()) = delay.poll_unpin(cx) {
						*state = ActiveSwapState::LockingTokens(
							call_lock_bridge_transfer_assets::<BFrom, BTo>(
								this.counterparty_contract.clone(),
								details.clone(),
							)
							.boxed(),
						);
					}
				}
				ActiveSwapState::WaitingForCompletedEvent => {}
				ActiveSwapState::CompletingBridging(_) => todo!(),
				ActiveSwapState::Completed => todo!(),
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
	mut counterparty_contract: BTo::CounterpartyContract,
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
