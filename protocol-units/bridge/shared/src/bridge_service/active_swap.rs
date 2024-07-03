use std::{
	collections::HashMap,
	convert::From,
	pin::Pin,
	task::{Context, Poll},
};

use futures::{task::AtomicWaker, Future, FutureExt, Stream};
use futures_timer::Delay;
use thiserror::Error;

use crate::bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator};
use crate::{
	blockchain_service::BlockchainService,
	bridge_contracts::{BridgeContractCounterpartyError, BridgeContractInitiatorError},
	types::{
		convert_bridge_transfer_id, BridgeTransferDetails, BridgeTransferId, CompletedDetails,
		HashLock,
	},
};

pub type BoxedFuture<R, E> = Pin<Box<dyn Future<Output = Result<R, E>> + Send>>;

pub struct ActiveSwap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	pub details: BridgeTransferDetails<BFrom::Address, BFrom::Hash>,
	pub state: ActiveSwapState<BTo>,
}

type Attempts = usize;

pub enum ActiveSwapState<BTo>
where
	BTo: BlockchainService,
{
	LockingTokens(BoxedFuture<(), LockBridgeTransferAssetsError>, Attempts),
	LockingTokensError(Delay, Attempts),
	WaitingForUnlockedEvent,
	CompletingBridging(
		BoxedFuture<(), CompleteBridgeTransferError>,
		CompletedDetails<BTo::Address, BTo::Hash>,
		Attempts,
	),
	CompletingBridgingError(Delay, CompletedDetails<BTo::Address, BTo::Hash>, Attempts),
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
	waker: AtomicWaker,
}

#[derive(Debug, Error)]
pub enum ActiveSwapMapError {
	#[error("Non existing swap")]
	NonExistingSwap,
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
			waker: AtomicWaker::new(),
		}
	}

	pub fn get(&self, key: &BridgeTransferId<BFrom::Hash>) -> Option<&ActiveSwap<BFrom, BTo>> {
		self.swaps.get(key)
	}

	pub fn get_mut(
		&mut self,
		key: &BridgeTransferId<BFrom::Hash>,
	) -> Option<&mut ActiveSwap<BFrom, BTo>> {
		self.swaps.get_mut(key)
	}

	pub fn already_executing(&self, key: &BridgeTransferId<BFrom::Hash>) -> bool {
		self.swaps.contains_key(key)
	}

	pub fn start_bridge_transfer(
		&mut self,
		details: BridgeTransferDetails<BFrom::Address, BFrom::Hash>,
	) where
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
					0,
				),
			},
		);

		self.waker.wake();
	}

	pub fn complete_bridge_transfer(
		&mut self,
		details: CompletedDetails<BTo::Address, BTo::Hash>,
	) -> Result<(), ActiveSwapMapError>
	where
		<BFrom as BlockchainService>::Hash: From<<BTo as BlockchainService>::Hash>,
		<<BFrom as BlockchainService>::InitiatorContract as BridgeContractInitiator>::Hash:
			From<<BTo as BlockchainService>::Hash>,
	{
		let active_swap = self
			.swaps
			.get_mut(&convert_bridge_transfer_id(details.bridge_transfer_id.clone()))
			.ok_or(ActiveSwapMapError::NonExistingSwap)?;

		debug_assert!(matches!(active_swap.state, ActiveSwapState::WaitingForUnlockedEvent));

		let initiator_contract = self.initiator_contract.clone();

		tracing::trace!(
			"Completing active swap for bridge transfer {:?}",
			details.bridge_transfer_id
		);

		active_swap.state = ActiveSwapState::CompletingBridging(
			call_complete_bridge_transfer::<BFrom, BTo>(initiator_contract, details.clone())
				.boxed(),
			details.clone(),
			self.config.error_attempts,
		);

		self.waker.wake();

		Ok(())
	}
}

#[derive(Debug)]
pub enum ActiveSwapEvent<H> {
	BridgeAssetsLocked(BridgeTransferId<H>),
	BridgeAssetsLockingError(LockBridgeTransferAssetsError),
	BridgeAssetsCompleted(BridgeTransferId<H>),
	BridgeAssetsCompletingError(CompleteBridgeTransferError),
}

impl<BFrom, BTo> Stream for ActiveSwapMap<BFrom, BTo>
where
	BFrom: BlockchainService + Unpin + 'static,

	<BFrom::InitiatorContract as BridgeContractInitiator>::Hash:
		From<<BTo as BlockchainService>::Hash>,
	<BFrom::InitiatorContract as BridgeContractInitiator>::Address:
		From<<BTo as BlockchainService>::Address>,

	BTo: BlockchainService + Unpin + 'static,

	<BTo::CounterpartyContract as BridgeContractCounterparty>::Hash: From<BFrom::Hash>,
	<BTo::CounterpartyContract as BridgeContractCounterparty>::Address: From<BFrom::Address>,
{
	type Item = ActiveSwapEvent<BFrom::Hash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> where {
		let this = self.get_mut();

		let mut cleanup: Vec<BridgeTransferId<BFrom::Hash>> = Vec::new();
		for (bridge_transfer_id, ActiveSwap { details: bridge_transfer, state, .. }) in
			this.swaps.iter_mut()
		{
			use ActiveSwapState::*;
			match state {
				LockingTokens(future, attempts) => {
					match future.poll_unpin(cx) {
						Poll::Ready(Ok(())) => {
							*state = ActiveSwapState::WaitingForUnlockedEvent;

							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsLocked(
								bridge_transfer_id.clone(),
							)));
						}
						Poll::Ready(Err(error)) => {
							// Locking tokens failed
							// Transition to the next state
							*state = ActiveSwapState::LockingTokensError(
								Delay::new(this.config.error_delay),
								*attempts,
							);
							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsLockingError(
								error,
							)));
						}
						Poll::Pending => {}
					}
				}
				LockingTokensError(delay, attempts) => {
					// test if the delay has expired
					// if it has, retry the lock
					if let Poll::Ready(()) = delay.poll_unpin(cx) {
						*state = ActiveSwapState::LockingTokens(
							call_lock_bridge_transfer_assets::<BFrom, BTo>(
								this.counterparty_contract.clone(),
								bridge_transfer.clone(),
							)
							.boxed(),
							*attempts + 1,
						);
					}
				}
				WaitingForUnlockedEvent => {
					continue;
				}
				CompletingBridging(future, details, attempts) => {
					match future.poll_unpin(cx) {
						Poll::Ready(Ok(())) => {
							*state = ActiveSwapState::Completed;

							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsCompleted(
								bridge_transfer_id.clone(),
							)));
						}
						Poll::Ready(Err(error)) => {
							// Completing bridging failed
							// Transition to the next state
							*state = ActiveSwapState::CompletingBridgingError(
								Delay::new(this.config.error_delay),
								details.clone(),
								*attempts + 1,
							);
							return Poll::Ready(Some(
								ActiveSwapEvent::BridgeAssetsCompletingError(error),
							));
						}
						Poll::Pending => {}
					}
				}
				CompletingBridgingError(delay, details, attempts) => {
					// test if the delay has expired
					// if it has, retry the lock
					if let Poll::Ready(()) = delay.poll_unpin(cx) {
						*state = ActiveSwapState::CompletingBridging(
							call_complete_bridge_transfer::<BFrom, BTo>(
								this.initiator_contract.clone(),
								details.clone(),
							)
							.boxed(),
							details.clone(),
							*attempts + 1,
						);
					}
				}
				Completed => {
					cleanup.push(bridge_transfer_id.clone());
				}
			}
		}

		// cleanup completed swaps
		for bridge_transfer_id in cleanup {
			this.swaps.remove(&bridge_transfer_id);
		}

		this.waker.register(cx.waker());

		Poll::Pending
	}
}

// Lock assets

#[derive(Debug, Error)]
pub enum LockBridgeTransferAssetsError {
	#[error("Failed to lock assets")]
	LockingError,
	#[error(transparent)]
	ContractCallError(#[from] BridgeContractCounterpartyError),
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
		.await?;

	Ok(())
}

#[derive(Debug, Error)]
pub enum CompleteBridgeTransferError {
	#[error("Failed to complete bridge transfer")]
	CompletingError,
	#[error(transparent)]
	ContractCallError(#[from] BridgeContractInitiatorError), // TODO; addd contact call errors
}

async fn call_complete_bridge_transfer<BFrom: BlockchainService, BTo: BlockchainService>(
	mut initiator_contract: BFrom::InitiatorContract,
	CompletedDetails { bridge_transfer_id, secret, .. }: CompletedDetails<BTo::Address, BTo::Hash>,
) -> Result<(), CompleteBridgeTransferError>
where
	<<BFrom as BlockchainService>::InitiatorContract as BridgeContractInitiator>::Hash:
		std::convert::From<<BTo as BlockchainService>::Hash>,
{
	tracing::trace!(
		"Calling complete bridge transfer on initiator contract for bridge transfer {:?}",
		bridge_transfer_id
	);

	initiator_contract
		.complete_bridge_transfer(convert_bridge_transfer_id(bridge_transfer_id), secret)
		.await?;

	Ok(())
}
