use futures::{Stream, StreamExt};
use std::task::{Context, Poll};
use std::{convert::From, pin::Pin};
use tracing::{trace, warn};

use crate::{
	blockchain_service::{BlockchainService, ContractEvent},
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
	bridge_service::{
		active_swap::ActiveSwapEvent,
		events::{CEvent, CWarn, IEvent, IWarn},
	},
	types::BridgeTransferId,
};

pub mod active_swap;
pub mod events;

use self::{
	active_swap::{ActiveSwapConfig, ActiveSwapMap},
	events::Event,
};

pub struct BridgeServiceConfig {
	pub active_swap: ActiveSwapConfig,
}

pub struct BridgeService<B1, B2, V>
where
	B1: BlockchainService,
	B2: BlockchainService,
{
	pub blockchain_1: B1,
	pub blockchain_2: B2,

	pub active_swaps_b1_to_b2: ActiveSwapMap<B1, B2, V>,
	pub active_swaps_b2_to_b1: ActiveSwapMap<B2, B1, V>,
}

impl<B1, B2, V> BridgeService<B1, B2, V>
where
	B1: BlockchainService + 'static,
	B2: BlockchainService + 'static,
	Vec<u8>: From<B1::Address>,
	Vec<u8>: From<B2::Address>,
{
	pub fn new(blockchain_1: B1, blockchain_2: B2, config: BridgeServiceConfig) -> Self {
		Self {
			active_swaps_b1_to_b2: ActiveSwapMap::build(
				blockchain_1.initiator_contract().clone(),
				blockchain_2.counterparty_contract().clone(),
				config.active_swap.clone(),
			),
			active_swaps_b2_to_b1: ActiveSwapMap::build(
				blockchain_2.initiator_contract().clone(),
				blockchain_1.counterparty_contract().clone(),
				config.active_swap.clone(),
			),
			blockchain_1,
			blockchain_2,
		}
	}
}

fn handle_initiator_event<BFrom, BTo, V>(
	initiator_event: BridgeContractInitiatorEvent<BFrom::Address, BFrom::Hash, V>,
	active_swaps: &mut ActiveSwapMap<BFrom, BTo, V>,
) -> Option<IEvent<BFrom::Address, BFrom::Hash, V>>
where
	BFrom: BlockchainService + 'static,
	BTo: BlockchainService + 'static,
	BTo::Hash: From<BFrom::Hash>,
	BTo::Address: From<Vec<u8>>,

	Vec<u8>: From<BTo::Address>,
	Vec<u8>: From<BFrom::Address>,
	V:Clone,
{
	match initiator_event {
		BridgeContractInitiatorEvent::Initiated(ref details) => {
			if active_swaps.already_executing(&details.bridge_transfer_id) {
				warn!("BridgeService: Bridge transfer {:?} already present, monitoring should only return event once", details.bridge_transfer_id);
				return Some(IEvent::Warn(IWarn::AlreadyPresent(details.clone())));
			}
			active_swaps.start_bridge_transfer(details.clone());
			Some(IEvent::ContractEvent(initiator_event))
		}
		BridgeContractInitiatorEvent::Completed(_) => Some(IEvent::ContractEvent(initiator_event)),
		BridgeContractInitiatorEvent::Refunded(_) => todo!(),
	}
}

fn handle_counterparty_event<BFrom, BTo, V>(
	event: BridgeContractCounterpartyEvent<BTo::Address, BTo::Hash, V>,
	active_swaps: &mut ActiveSwapMap<BFrom, BTo, V>,
) -> Option<CEvent<BTo::Address, BTo::Hash, V>>
where
	BFrom: BlockchainService + 'static,
	BTo: BlockchainService + 'static,
	BFrom::Hash: From<BTo::Hash>,

	Vec<u8>: From<BTo::Address>,
	Vec<u8>: From<BFrom::Address>,
	V:Clone,
{
	use BridgeContractCounterpartyEvent::*;
	match event {
		Locked(ref _details) => Some(CEvent::ContractEvent(event)),
		Completed(ref details) => match active_swaps.complete_bridge_transfer(details.clone()) {
			Ok(_) => {
				trace!("BridgeService: Bridge transfer completed successfully");
				Some(CEvent::ContractEvent(event))
			}
			Err(error) => {
				warn!("BridgeService: Error completing bridge transfer: {:?}", error);
				match error {
					active_swap::ActiveSwapMapError::NonExistingSwap => {
						Some(CEvent::Warn(CWarn::CannotCompleteUnexistingSwap(details.clone())))
					}
				}
			}
		},
	}
}

impl<B1, B2, V> Stream for BridgeService<B1, B2, V>
where
	B1: BlockchainService + 'static,
	B2: BlockchainService + 'static,

	B1::Hash: From<B2::Hash>,
	B2::Hash: From<B1::Hash>,

	B1::Address: From<Vec<u8>>,
	B2::Address: From<Vec<u8>>,

	Vec<u8>: From<B1::Address>,
	Vec<u8>: From<B2::Address>,

	V: Clone + Unpin,
{
	type Item = Event<B1, B2, V>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		// Poll the active swaps in both directions and return the appropriate events
		{
			use HandleActiveSwapEvent::*;

			let active_swap_event = this.active_swaps_b1_to_b2.poll_next_unpin(cx);
			if let Some(value) = handle_active_swap_event::<B1, B2, V>(active_swap_event) {
				match value {
					InitiatorEvent(event) => return Poll::Ready(Some(Event::B1I(event))),
					CounterpartyEvent(event) => return Poll::Ready(Some(Event::B2C(event))),
				}
			}

			let active_swap_event = this.active_swaps_b2_to_b1.poll_next_unpin(cx);
			if let Some(value) = handle_active_swap_event::<B2, B1, V>(active_swap_event) {
				match value {
					InitiatorEvent(event) => return Poll::Ready(Some(Event::B2I(event))),
					CounterpartyEvent(event) => return Poll::Ready(Some(Event::B1C(event))),
				}
			}
		}

		// Poll the bridge services, handle the appropriate events, and return
		match this.blockchain_1.poll_next_unpin(cx) {
			Poll::Ready(Some(blockchain_event)) => {
				trace!(
					"BridgeService: Received event from blockchain service 1: {:?}",
					blockchain_event
				);
				match blockchain_event {
					ContractEvent::InitiatorEvent(initiator_event) => {
						trace!("BridgeService: Initiator event from blockchain service 1");
						if let Some(propagate_event) = handle_initiator_event::<B1, B2, V>(
							initiator_event,
							&mut this.active_swaps_b1_to_b2,
						) {
							return Poll::Ready(Some(Event::B1I(propagate_event)));
						}
					}
					ContractEvent::CounterpartyEvent(counterparty_event) => {
						if let Some(propagate_event) = handle_counterparty_event::<B2, B1, V>(
							counterparty_event,
							&mut this.active_swaps_b2_to_b1,
						) {
							return Poll::Ready(Some(Event::B1C(propagate_event)));
						}
						trace!("BridgeService: Counterparty event from blockchain service 1");
					}
				}
			}
			Poll::Ready(None) => {
				trace!("BridgeService: Blockchain service 1 has no more events");
			}
			Poll::Pending => {
				trace!("BridgeService: Blockchain service 1 has no events at this time");
			}
		}

		match this.blockchain_2.poll_next_unpin(cx) {
			Poll::Ready(Some(blockchain_event)) => {
				trace!(
					"BridgeService: Received event from blockchain service 2: {:?}",
					blockchain_event
				);
				match blockchain_event {
					ContractEvent::InitiatorEvent(initiator_event) => {
						trace!("BridgeService: Initiator event from blockchain service 2");
						if let Some(propagate_event) = handle_initiator_event::<B2, B1, V>(
							initiator_event,
							&mut this.active_swaps_b2_to_b1,
						) {
							return Poll::Ready(Some(Event::B2I(propagate_event)));
						}
					}
					ContractEvent::CounterpartyEvent(counterparty_event) => {
						trace!("BridgeService: Counterparty event from blockchain service 2");
						if let Some(propagate_event) = handle_counterparty_event::<B1, B2, V>(
							counterparty_event,
							&mut this.active_swaps_b1_to_b2,
						) {
							return Poll::Ready(Some(Event::B2C(propagate_event)));
						}
					}
				}
			}
			Poll::Ready(None) => {
				trace!("BridgeService: Blockchain service 2 has no more events");
			}
			Poll::Pending => {
				trace!("BridgeService: Blockchain service 2 has no events at this time");
			}
		}

		Poll::Pending
	}
}

// Initiator events pertain to the initiator contract, while counterparty events are associated
// with the counterparty contract.
enum HandleActiveSwapEvent<BFrom, BTo, V>
where
	BFrom: BlockchainService,
	BTo: BlockchainService,
{
	InitiatorEvent(IEvent<BFrom::Address, BFrom::Hash, V>),
	CounterpartyEvent(CEvent<BTo::Address, BTo::Hash, V>),
}

fn handle_active_swap_event<BFrom, BTo, V>(
	active_swap_event: Poll<Option<ActiveSwapEvent<BFrom::Hash>>>,
) -> Option<HandleActiveSwapEvent<BFrom, BTo, V>>
where
	BFrom: BlockchainService + 'static,
	BTo: BlockchainService + 'static,
	BTo::Hash: From<BFrom::Hash>,
	<BTo as BlockchainService>::Hash: BlockchainService,
{
	use ActiveSwapEvent::*;
	match active_swap_event {
		Poll::Ready(Some(event)) => {
			trace!("BridgeService: Received event from active swaps: {:?}", event);
			match event {
				// Locking
				BridgeAssetsLocked(bridge_transfer_id) => {
					trace!(
						"BridgeService: Bridge assets locked for transfer {:?}",
						bridge_transfer_id
					);
				}
				BridgeAssetsLockingError(error) => {
					// The error in locking bridge assets occurs when transitioning from blockchain 1 to blockchain 2.
					// This issue arises during the attempt to communicate with blockchain 2 for accessing the locked funds.
					// Hence the Event::B2C
					warn!("BridgeService: Error locking bridge assets: {:?}", error);
					return Some(HandleActiveSwapEvent::CounterpartyEvent::<<BFrom as BlockchainService>::Address,<BTo as BlockchainService>::Hash, V>(CEvent::Warn(
						CWarn::BridgeAssetsLockingError(error),
					)));
				}
				BridgeAssetsRetryLocking(bridge_transfer_id) => {
					warn!(
						"BridgeService: Retrying to lock bridge assets for transfer {:?}",
						bridge_transfer_id
					);
					return Some(HandleActiveSwapEvent::CounterpartyEvent(
						CEvent::RetryLockingAssets(BridgeTransferId(From::from(
							bridge_transfer_id.0,
						))),
					));
				}
				BridgeAssetsLockingAbortedTooManyAttempts(bride_transfer_id) => {
					warn!(
						"BridgeService: Aborted bridge transfer due to too many attempts: {:?}",
						bride_transfer_id
					);
					return Some(HandleActiveSwapEvent::CounterpartyEvent(CEvent::Warn(
						CWarn::LockingAbortedTooManyAttempts(BridgeTransferId(From::from(
							bride_transfer_id.0,
						))),
					)));
				}

				// Completing
				BridgeAssetsCompleted(bridge_transfer_id) => {
					trace!(
						"BridgeService: Bridge assets completed for transfer {:?}",
						bridge_transfer_id
					);
				}
				BridgeAssetsCompletingError(bridge_transfer_id, error) => {
					warn!("BridgeService: Error completing bridge assets: {:?}", error);
					return Some(HandleActiveSwapEvent::InitiatorEvent(IEvent::Warn(
						IWarn::CompleteTransferError(bridge_transfer_id.clone()),
					)));
				}

				BridgeAssetsRetryCompleting(bridge_transfer_id) => {
					warn!(
						"BridgeService: Retrying to complete bridge assets for transfer {:?}",
						bridge_transfer_id
					);
					return Some(HandleActiveSwapEvent::InitiatorEvent(
						IEvent::RetryCompletingTransfer(bridge_transfer_id),
					));
				}

				BridgeAssetsCompletingAbortedTooManyAttempts(bridge_transfer_id) => {
					warn!(
						"BridgeService: Aborted bridge transfer completion due to too many errors: {:?}",
						bridge_transfer_id
					);
					return Some(HandleActiveSwapEvent::InitiatorEvent(IEvent::Warn(
						IWarn::CompletionAbortedTooManyAttempts(bridge_transfer_id),
					)));
				}
			}
		}
		Poll::Ready(None) => {
			trace!("BridgeService: Active swaps has no more events");
		}
		Poll::Pending => {
			trace!("BridgeService: Active swaps has no events at this time");
		}
	}
	None
}
