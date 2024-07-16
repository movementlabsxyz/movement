use std::{
	collections::HashMap,
	convert::From,
	pin::Pin,
	task::{Context, Poll},
	time::Duration,
};

use futures::{task::AtomicWaker, Future, FutureExt, Stream};
use futures_time::future::{FutureExt as TimeoutFutureExt, Timeout};
use futures_timer::Delay;
use thiserror::Error;

use crate::{
	blockchain_service::BlockchainService,
	bridge_contracts::{BridgeContractCounterpartyError, BridgeContractInitiatorError},
	types::{
		convert_bridge_transfer_id, BridgeTransferDetails, BridgeTransferId,
		CounterpartyCompletedDetails, HashLock, InitiatorAddress,
	},
};
use crate::{
	bridge_contracts::{BridgeContractCounterparty, BridgeContractInitiator},
	types::RecipientAddress,
};

pub type BoxedFuture<R, E> = Timeout<Pin<Box<dyn Future<Output = Result<R, E>> + Send>>, Delay>;

pub struct ActiveSwap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	pub details: BridgeTransferDetails<BFrom::Address, BFrom::Hash>,
	pub state: ActiveSwapState<BTo>,
}

impl<BFrom, BTo> std::fmt::Debug for ActiveSwap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ActiveSwap")
			.field("details", &self.details)
			.field("state", &self.state)
			.finish()
	}
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
		CounterpartyCompletedDetails<BTo::Address, BTo::Hash>,
		Attempts,
	),
	CompletingBridgingError(Delay, CounterpartyCompletedDetails<BTo::Address, BTo::Hash>, Attempts),
	Completed,
	Aborted,
}

impl<BTo> std::fmt::Debug for ActiveSwapState<BTo>
where
	BTo: BlockchainService,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ActiveSwapState::LockingTokens(_, attempts) => {
				f.debug_struct("LockingTokens").field("attempts", attempts).finish()
			}
			ActiveSwapState::LockingTokensError(_, attempts) => {
				f.debug_struct("LockingTokensError").field("attempts", attempts).finish()
			}
			ActiveSwapState::WaitingForUnlockedEvent => {
				f.debug_tuple("WaitingForUnlockedEvent").finish()
			}
			ActiveSwapState::CompletingBridging(_, _, attempts) => {
				f.debug_struct("CompletingBridging").field("attempts", attempts).finish()
			}
			ActiveSwapState::CompletingBridgingError(_, _, attempts) => {
				f.debug_struct("CompletingBridgingError").field("attempts", attempts).finish()
			}
			ActiveSwapState::Completed => f.debug_tuple("Completed").finish(),
			ActiveSwapState::Aborted => f.debug_tuple("Aborted").finish(),
		}
	}
}

#[derive(Debug, Clone)]
pub struct ActiveSwapConfig {
	pub error_attempts: usize,
	pub error_delay: Duration,
	pub contract_call_timeout: Duration,
}
impl Default for ActiveSwapConfig {
	fn default() -> Self {
		Self {
			error_attempts: 3,
			error_delay: Duration::from_secs(5),
			contract_call_timeout: Duration::from_secs(30),
		}
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

impl<BFrom, BTo> std::fmt::Debug for ActiveSwapMap<BFrom, BTo>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ActiveSwapMap")
			.field("swaps", &self.swaps)
			.field("config", &self.config)
			.finish()
	}
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
	Vec<u8>: From<BTo::Address>,
	Vec<u8>: From<BFrom::Address>,
{
	pub fn build(
		initiator_contract: BFrom::InitiatorContract,
		counterparty_contract: BTo::CounterpartyContract,
		config: ActiveSwapConfig,
	) -> Self {
		Self {
			initiator_contract,
			counterparty_contract,
			swaps: HashMap::new(),
			config,
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
		BTo::Address: From<Vec<u8>>,
		BTo::Hash: From<BFrom::Hash>,
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
						.boxed()
						.timeout(Delay::new(self.config.contract_call_timeout)),
					0,
				),
			},
		);

		self.waker.wake();
	}

	pub fn complete_bridge_transfer(
		&mut self,
		details: CounterpartyCompletedDetails<BTo::Address, BTo::Hash>,
	) -> Result<(), ActiveSwapMapError>
	where
		BFrom::Hash: From<BTo::Hash>,
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
				.boxed()
				.timeout(Delay::new(self.config.contract_call_timeout)),
			details.clone(),
			0,
		);

		self.waker.wake();

		Ok(())
	}
}

#[derive(Debug)]
pub enum ActiveSwapEvent<H> {
	BridgeAssetsLocked(BridgeTransferId<H>),
	BridgeAssetsLockingError(LockBridgeTransferAssetsError),
	BridgeAssetsRetryLocking(BridgeTransferId<H>),
	BridgeAssetsCompleted(BridgeTransferId<H>),
	BridgeAssetsCompletingError(BridgeTransferId<H>, CompleteBridgeTransferError),
	BridgeAssetsRetryCompleting(BridgeTransferId<H>),
	BridgeAssetsLockingAbortedTooManyAttempts(BridgeTransferId<H>),
	BridgeAssetsCompletingAbortedTooManyAttempts(BridgeTransferId<H>),
}

fn catch_timeout_error<T, E: HasTimeoutError>(
	result: Poll<Result<Result<T, E>, std::io::Error>>,
) -> Poll<Result<T, E>> {
	match result {
		Poll::Ready(Ok(result)) => Poll::Ready(result),
		Poll::Ready(Err(_)) => Poll::Ready(Err(E::timeout_error())),
		Poll::Pending => Poll::Pending,
	}
}

impl<BFrom, BTo> Stream for ActiveSwapMap<BFrom, BTo>
where
	BFrom: BlockchainService + 'static,
	BTo: BlockchainService + 'static,

	BFrom::Hash: From<BTo::Hash>,
	BTo::Hash: From<BFrom::Hash>,

	BTo::Address: From<Vec<u8>>,

	Vec<u8>: From<BTo::Address>,
	Vec<u8>: From<BFrom::Address>,
{
	type Item = ActiveSwapEvent<BFrom::Hash>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		tracing::trace!("Polling active swap map");

		// remove all swaps that are completed or aborted
		this.swaps.retain(|_, swap| {
			!matches!(swap.state, ActiveSwapState::Completed | ActiveSwapState::Aborted)
		});

		for (bridge_transfer_id, ActiveSwap { details: bridge_transfer, state, .. }) in
			this.swaps.iter_mut()
		{
			use ActiveSwapState::*;
			match state {
				LockingTokens(future, attempts) => {
					tracing::trace!("Polling locking_tokens {:?}", bridge_transfer_id);
					match catch_timeout_error(future.poll_unpin(cx)) {
						Poll::Ready(Ok(())) => {
							*state = ActiveSwapState::WaitingForUnlockedEvent;

							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsLocked(
								bridge_transfer_id.clone(),
							)));
						}
						Poll::Ready(Err(error)) => {
							tracing::trace!(
								"Locking brige_transfer {:?} failed, error: {:?} attempts: {}",
								bridge_transfer_id,
								error,
								attempts
							);
							if *attempts >= this.config.error_attempts {
								*state = ActiveSwapState::Aborted;
								return Poll::Ready(Some(
									ActiveSwapEvent::BridgeAssetsLockingAbortedTooManyAttempts(
										bridge_transfer_id.clone(),
									),
								));
							}
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
						tracing::trace!(
							"Retrying lock for bridge transfer {:?}",
							bridge_transfer_id
						);
						*state = ActiveSwapState::LockingTokens(
							call_lock_bridge_transfer_assets::<BFrom, BTo>(
								this.counterparty_contract.clone(),
								bridge_transfer.clone(),
							)
							.boxed()
							.timeout(Delay::new(this.config.contract_call_timeout)),
							*attempts + 1,
						);
						return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsRetryLocking(
							bridge_transfer_id.clone(),
						)));
					}
				}
				WaitingForUnlockedEvent => {
					continue;
				}
				CompletingBridging(future, details, attempts) => {
					match catch_timeout_error(future.poll_unpin(cx)) {
						Poll::Ready(Ok(())) => {
							*state = ActiveSwapState::Completed;

							return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsCompleted(
								bridge_transfer_id.clone(),
							)));
						}
						Poll::Ready(Err(error)) => {
							tracing::trace!(
								"Completing bridge transfer {:?} failed: {:?} attemtps: {}",
								bridge_transfer_id,
								error,
								attempts
							);
							if *attempts >= this.config.error_attempts {
								*state = ActiveSwapState::Aborted;
								return Poll::Ready(Some(
									ActiveSwapEvent::BridgeAssetsCompletingAbortedTooManyAttempts(
										bridge_transfer_id.clone(),
									),
								));
							}

							// Completing bridging failed
							// Transition to the next state
							*state = ActiveSwapState::CompletingBridgingError(
								Delay::new(this.config.error_delay),
								details.clone(),
								*attempts + 1,
							);

							return Poll::Ready(Some(
								ActiveSwapEvent::BridgeAssetsCompletingError(
									bridge_transfer_id.clone(),
									error,
								),
							));
						}
						Poll::Pending => {}
					}
				}
				CompletingBridgingError(delay, details, attempts) => {
					tracing::trace!(
						"Retrying completing of bridge transfer {:?}",
						bridge_transfer_id
					);

					// test if the delay has expired
					// if it has, retry the lock
					if let Poll::Ready(()) = delay.poll_unpin(cx) {
						*state = ActiveSwapState::CompletingBridging(
							call_complete_bridge_transfer::<BFrom, BTo>(
								this.initiator_contract.clone(),
								details.clone(),
							)
							.boxed()
							.timeout(Delay::new(this.config.contract_call_timeout)),
							details.clone(),
							*attempts + 1,
						);
						return Poll::Ready(Some(ActiveSwapEvent::BridgeAssetsRetryCompleting(
							bridge_transfer_id.clone(),
						)));
					}
				}
				Completed => {
					tracing::trace!(
						"Bridge transfer {:?} completed, marked for cleanup",
						bridge_transfer_id
					);
				}
				Aborted => {
					tracing::trace!(
						"Bridge transfer {:?} aborted, marked for cleanup",
						bridge_transfer_id
					);
				}
			}
		}

		this.waker.register(cx.waker());

		Poll::Pending
	}
}

// Lock assets
trait HasTimeoutError {
	fn timeout_error() -> Self;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LockBridgeTransferAssetsError {
	#[error("Failed to lock assets")]
	LockingError,
	#[error("Timeout while performing contract call")]
	ContractCallTimeoutError,
	#[error(transparent)]
	ContractCallError(#[from] BridgeContractCounterpartyError),
}

impl HasTimeoutError for LockBridgeTransferAssetsError {
	fn timeout_error() -> Self {
		LockBridgeTransferAssetsError::ContractCallTimeoutError
	}
}

async fn call_lock_bridge_transfer_assets<BFrom: BlockchainService, BTo: BlockchainService>(
	mut counterparty_contract: BTo::CounterpartyContract,
	BridgeTransferDetails {
		bridge_transfer_id,
		hash_lock,
		time_lock,
		recipient_address,
		initiator_address,
		amount,
		..
	}: BridgeTransferDetails<BFrom::Address, BFrom::Hash>,
) -> Result<(), LockBridgeTransferAssetsError>
where
	BTo::Address: From<Vec<u8>>,
	BTo::Hash: From<BFrom::Hash>,
	Vec<u8>: From<BTo::Address>,
	Vec<u8>: From<BFrom::Address>,
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
			InitiatorAddress(From::from(initiator_address.0)),
			RecipientAddress(From::from(recipient_address.0)),
			amount,
		)
		.await?;

	Ok(())
}

#[derive(Debug, Error)]
pub enum CompleteBridgeTransferError {
	#[error("Failed to complete bridge transfer")]
	CompletingError,
	#[error("Timeout while performing contract call")]
	ContractCallTimeoutError,
	#[error(transparent)]
	ContractCallError(#[from] BridgeContractInitiatorError),
}

impl HasTimeoutError for CompleteBridgeTransferError {
	fn timeout_error() -> Self {
		CompleteBridgeTransferError::ContractCallTimeoutError
	}
}

async fn call_complete_bridge_transfer<BFrom: BlockchainService, BTo: BlockchainService>(
	mut initiator_contract: BFrom::InitiatorContract,
	CounterpartyCompletedDetails { bridge_transfer_id, secret, .. }: CounterpartyCompletedDetails<
		BTo::Address,
		BTo::Hash,
	>,
) -> Result<(), CompleteBridgeTransferError>
where
	BFrom::Hash: From<BTo::Hash>,
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
