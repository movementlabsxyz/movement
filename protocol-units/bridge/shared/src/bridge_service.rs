use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::blockchain_service::{BlockchainEvent, BlockchainService};

pub struct BridgeService<B1, B2>
where
	B1: BlockchainService,
	B2: BlockchainService,
{
	blockchain_1: B1,
	blockchain_2: B2,
}

impl<B1, B2> BridgeService<B1, B2>
where
	B1: BlockchainService,
	B2: BlockchainService,
{
	pub fn new(blockchain_1: B1, blockchain_2: B2) -> Self {
		Self { blockchain_1, blockchain_2 }
	}
}

impl<B1, B2> Stream for BridgeService<B1, B2>
where
	B1: BlockchainService + Unpin,
	B2: BlockchainService + Unpin,
{
	type Item = ();

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		match this.blockchain_1.poll_next_unpin(cx) {
			Poll::Ready(Some(event)) => {
				println!("BridgeService: Received event from blockchain service 1: {:?}", event);
				match event {
					BlockchainEvent::InitiatorEvent(_) => {
						println!("BridgeService: Initiator event from blockchain service 1");
					}
					BlockchainEvent::CounterpartyEvent(_) => {
						println!("BridgeService: Counterparty event from blockchain service 1");
					}
				}
			}
			Poll::Ready(None) => {
				println!("BridgeService: Blockchain service 1 has no more events");
			}
			Poll::Pending => {
				println!("BridgeService: Blockchain service 1 has no events at this time");
			}
		}

		match this.blockchain_2.poll_next_unpin(cx) {
			Poll::Ready(Some(event)) => {
				println!("BridgeService: Received event from blockchain service 2: {:?}", event);
				match event {
					BlockchainEvent::InitiatorEvent(_) => {
						println!("BridgeService: Initiator event from blockchain service 2");
					}
					BlockchainEvent::CounterpartyEvent(_) => {
						println!("BridgeService: Counterparty event from blockchain service 2");
					}
				}
			}
			Poll::Ready(None) => {
				println!("BridgeService: Blockchain service 2 has no more events");
			}
			Poll::Pending => {
				println!("BridgeService: Blockchain service 2 has no events at this time");
			}
		}

		Poll::Pending
	}
}
