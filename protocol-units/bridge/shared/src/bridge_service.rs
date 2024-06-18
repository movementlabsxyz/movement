use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{trace, warn};

use crate::{
	blockchain_service::{BlockchainService, ContractEvent},
	bridge_contracts::BridgeContractCounterparty,
	bridge_monitoring::{BridgeContractCounterpartyEvent, BridgeContractInitiatorEvent},
	types::BridgeTransferDetails,
};

pub mod active_swap;

use self::active_swap::ActiveSwapMap;

pub struct BridgeService<B1, B2>
where
	B1: BlockchainService,
	B2: BlockchainService,
{
	pub blockchain_1: B1,
	pub blockchain_2: B2,

	pub active_swaps_b1_to_b2: ActiveSwapMap<B1, B2>,
	pub active_swaps_b2_to_b1: ActiveSwapMap<B2, B1>,
}

impl<B1, B2> BridgeService<B1, B2>
where
	B1: BlockchainService + 'static,
	B2: BlockchainService + 'static,
{
	pub fn new(blockchain_1: B1, blockchain_2: B2) -> Self {
		Self {
			active_swaps_b1_to_b2: ActiveSwapMap::build(
				blockchain_1.initiator_contract().clone(),
				blockchain_2.counterparty_contract().clone(),
			),
			active_swaps_b2_to_b1: ActiveSwapMap::build(
				blockchain_2.initiator_contract().clone(),
				blockchain_1.counterparty_contract().clone(),
			),
			blockchain_1,
			blockchain_2,
		}
	}
}

#[derive(Debug)]
pub enum IWarn<B: BlockchainService> {
	AlreadyPresent(BridgeTransferDetails<B::Address, B::Hash>),
}

#[derive(Debug)]
pub enum IEvent<B: BlockchainService> {
	ContractEvent(BridgeContractInitiatorEvent<B::Address, B::Hash>),
	Warn(IWarn<B>),
}

#[derive(Debug)]
pub enum CEvent<B: BlockchainService> {
	ContractEvent(BridgeContractCounterpartyEvent<B::Address, B::Hash>),
}

#[derive(Debug)]
pub enum Event<B1, B2>
where
	B1: BlockchainService,
	B2: BlockchainService,
{
	B1I(IEvent<B1>),
	B1C(CEvent<B1>),
	B2I(IEvent<B2>),
	B2C(CEvent<B2>),
}

impl<B1, B2> Stream for BridgeService<B1, B2>
where
	B1: BlockchainService + Unpin + 'static,
	B2: BlockchainService + Unpin + 'static,

	<B2::CounterpartyContract as BridgeContractCounterparty>::Hash: From<B1::Hash>,
	<B2::CounterpartyContract as BridgeContractCounterparty>::Address: From<B1::Address>,

	<B1::CounterpartyContract as BridgeContractCounterparty>::Hash: From<B2::Hash>,
	<B1::CounterpartyContract as BridgeContractCounterparty>::Address: From<B2::Address>,
{
	type Item = Event<B1, B2>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		match this.active_swaps_b1_to_b2.poll_next_unpin(cx) {
			Poll::Ready(Some(event)) => {
				trace!("BridgeService: Received event from active swaps B1 -> B2: {:?}", event);
				match event {
					active_swap::ActiveSwapEvent::BridgeAssetsLocked(bridge_transfer_id) => {
						trace!(
							"BridgeService: Bridge assets locked for transfer {:?}",
							bridge_transfer_id
						);
						// The smart contract has been called on blockchain_2. Now, we have to wait for
						// confirmation from the blockchain_2 event.
					}
					active_swap::ActiveSwapEvent::BridgeAssetsLockingError(error) => {
						warn!("BridgeService: Error locking bridge assets: {:?}", error);
						// An error occurred while calling the lock_bridge_transfer_assets method. This
						// could be due to a network error or an issue with the smart contract call.

						// We should retry this active swap for a number of times before giving up, and
						// otherwise refund the bridge transfer.
					}
				}
			}
			Poll::Ready(None) => {
				trace!("BridgeService: Active swaps B1 -> B2 has no more events");
			}
			Poll::Pending => {
				trace!("BridgeService: Active swaps B1 -> B2 has no events at this time");
			}
		}

		match this.active_swaps_b2_to_b1.poll_next_unpin(cx) {
			Poll::Ready(Some(event)) => {
				trace!("BridgeService: Received event from active swaps B2 -> B1: {:?}", event);
				match event {
					active_swap::ActiveSwapEvent::BridgeAssetsLocked(bridge_transfer_id) => {
						trace!(
							"BridgeService: Bridge assets locked for transfer {:?}",
							bridge_transfer_id
						);
					}
					active_swap::ActiveSwapEvent::BridgeAssetsLockingError(error) => {
						warn!("BridgeService: Error locking bridge assets: {:?}", error);
					}
				}
			}
			Poll::Ready(None) => {
				trace!("BridgeService: Active swaps B2 -> B1 has no more events");
			}
			Poll::Pending => {
				trace!("BridgeService: Active swaps B2 -> B1 has no events at this time");
			}
		}

		use Event::*;

		match this.blockchain_1.poll_next_unpin(cx) {
			Poll::Ready(Some(event)) => {
				trace!("BridgeService: Received event from blockchain service 1: {:?}", event);
				match event {
					ContractEvent::InitiatorEvent(initiator_event) => {
						trace!("BridgeService: Initiator event from blockchain service 1");
						match initiator_event {
							BridgeContractInitiatorEvent::BridgeTransferInitiated(ref details) => {
								// Bridge transfer initiated. Now, as the counterparty, we should lock
								// the appropriate tokens using the same secret.
								if this
									.active_swaps_b1_to_b2
									.already_executing(&details.bridge_transfer_id)
								{
									warn!("BridgeService: Bridge transfer {:?} already present, monitoring should only return event once", details.bridge_transfer_id);
									return Poll::Ready(Some(B1I(IEvent::Warn(
										IWarn::AlreadyPresent(details.clone()),
									))));
								}

								this.active_swaps_b1_to_b2.start(details.clone());
								return Poll::Ready(Some(B1I(IEvent::ContractEvent(
									initiator_event,
								))));
							}
							BridgeContractInitiatorEvent::BridgeTransferCompleted(_) => todo!(),
							BridgeContractInitiatorEvent::BridgeTransferRefunded(_) => todo!(),
						}
					}
					ContractEvent::CounterpartyEvent(_) => {
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
			Poll::Ready(Some(event)) => {
				trace!("BridgeService: Received event from blockchain service 2: {:?}", event);
				match event {
					ContractEvent::InitiatorEvent(_) => {
						trace!("BridgeService: Initiator event from blockchain service 2");
					}
					ContractEvent::CounterpartyEvent(_) => {
						trace!("BridgeService: Counterparty event from blockchain service 2");
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
