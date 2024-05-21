use crate::{BlockCommitmentEvent, CommitmentEventStream, McrSettlementManagerOperations};

use mcr_settlement_client::McrSettlementClientOperations;
use movement_types::{BlockCommitment, BlockCommitmentRejectionReason};

use async_stream::stream;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use std::collections::BTreeMap;
use std::mem;

/// Public handle for the MCR settlement manager.
pub struct Manager {
	sender: mpsc::Sender<BlockCommitment>,
}

impl Manager {
	/// Creates a new MCR settlement manager.
	///
	/// Returns the handle with the public API and the stream to receive commitment events.
	/// The stream needs to be polled to drive the MCR settlement client and
	/// process the commitments.
	pub fn new<C: McrSettlementClientOperations + Send + 'static>(
		client: C,
	) -> (Self, CommitmentEventStream) {
		let (sender, receiver) = mpsc::channel(16);
		let event_stream = process_commitments(receiver, client);
		(Self { sender }, event_stream)
	}
}

#[async_trait]
impl McrSettlementManagerOperations for Manager {
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {
		self.sender.send(block_commitment).await?;
		Ok(())
	}
}

fn process_commitments<C: McrSettlementClientOperations + Send + 'static>(
	mut receiver: mpsc::Receiver<BlockCommitment>,
	client: C,
) -> CommitmentEventStream {
	// Can't mix try_stream! and select!, see https://github.com/tokio-rs/async-stream/issues/63
	Box::pin(stream! {
		let mut settlement_stream = client.stream_block_commitments().await?;
		let mut max_height = client.get_max_tolerable_block_height().await?;
		let mut ahead_of_settlement = false;
		let mut commitments_to_settle = BTreeMap::new();
		let mut batch_acc = Vec::new();
		loop {
			tokio::select! {
				Some(block_commitment) = receiver.recv(), if !ahead_of_settlement => {
					commitments_to_settle.insert(
						block_commitment.height,
						block_commitment.commitment.clone(),
					);
					if block_commitment.height > max_height {
						ahead_of_settlement = true;
						let batch = mem::replace(&mut batch_acc, Vec::new());
						if let Err(e) = client.post_block_commitment_batch(batch).await {
							yield Err(e);
							break;
						}
					}
					batch_acc.push(block_commitment);
				}
				Some(res) = settlement_stream.next() => {
					let settled_commitment = match res {
						Ok(commitment) => commitment,
						Err(e) => {
							yield Err(e);
							break;
						}
					};

					let height = settled_commitment.height;
					if let Some(commitment) = commitments_to_settle.remove(&height) {
						let event = if commitment == settled_commitment.commitment {
							BlockCommitmentEvent::Accepted(settled_commitment)
						} else {
							BlockCommitmentEvent::Rejected {
								height,
								reason: BlockCommitmentRejectionReason::InvalidCommitment,
							}
						};
						yield Ok(event);
					} else if let Some((&lh, _)) = commitments_to_settle.last_key_value() {
						if lh < height {
							// Settlement has left some commitments behind, but the client could
							// deliver them of order?
							todo!("Handle falling behind on settlement")
						}
					}
					// Remove back-pressure if we can proceed settling new blocks.
					if ahead_of_settlement {
						let new_max_height = match client.get_max_tolerable_block_height().await {
							Ok(h) => h,
							Err(e) => {
								yield Err(e);
								break;
							}
						};
						if new_max_height > max_height {
							max_height = new_max_height;
							ahead_of_settlement = false;
						}
					}
				}
				else => break
			}
		}
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use mcr_settlement_client::mock::MockMcrSettlementClient;
	use movement_types::{BlockCommitment, Commitment};

	#[tokio::test]
	async fn test_block_commitment_accepted() -> Result<(), anyhow::Error> {
		let mut client = MockMcrSettlementClient::new();
		client.block_lead_tolerance = 1;
		let (manager, mut event_stream) = Manager::new(client.clone());
		let commitment = BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment([1; 32]),
		};
		manager.post_block_commitment(commitment.clone()).await?;
		let commitment2 = BlockCommitment {
			height: 2,
			block_id: Default::default(),
			commitment: Commitment([2; 32]),
		};
		manager.post_block_commitment(commitment2).await?;
		let item = event_stream.next().await;
		let res = item.unwrap();
		let event = res.unwrap();
		assert_eq!(event, BlockCommitmentEvent::Accepted(commitment));
		Ok(())
	}

	#[tokio::test]
	async fn test_block_commitment_rejected() -> Result<(), anyhow::Error> {
		let mut client = MockMcrSettlementClient::new();
		client.block_lead_tolerance = 1;
		let (manager, mut event_stream) = Manager::new(client.clone());
		let commitment = BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment([1; 32]),
		};
		client.settle(BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment([3; 32]),
		}).await;
		manager.post_block_commitment(commitment.clone()).await?;
		let commitment2 = BlockCommitment {
			height: 2,
			block_id: Default::default(),
			commitment: Commitment([2; 32]),
		};
		manager.post_block_commitment(commitment2).await?;
		let item = event_stream.next().await;
		let res = item.unwrap();
		let event = res.unwrap();
		assert_eq!(event, BlockCommitmentEvent::Rejected {
			height: 1,
			reason: BlockCommitmentRejectionReason::InvalidCommitment,
		});
		Ok(())
	}
}
