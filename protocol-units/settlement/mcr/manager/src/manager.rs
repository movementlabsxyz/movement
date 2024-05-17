use crate::{BlockCommitmentEvent, CommitmentEventStream, McrSettlementManagerOperations};

use mcr_settlement_client::McrSettlementClientOperations;
use movement_types::{BlockCommitment, BlockCommitmentRejectionReason};

use async_stream::try_stream;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use std::collections::BTreeMap;
use std::future::Future;
use std::mem;

/// Public handle for the MCR settlement manager.
pub struct Manager {
	sender: mpsc::Sender<BlockCommitment>,
}

struct BackgroundTask<C> {
	receiver: mpsc::Receiver<BlockCommitment>,
	client: C,
	event_sender: mpsc::Sender<BlockCommitmentEvent>,
}

impl Manager {
	/// Creates a new MCR settlement manager.
	///
	/// Returns the handle with the public API, the stream to receive commitment events,
	/// and a future that drives the background task.
	pub fn new<C: McrSettlementClientOperations + Send>(
		client: C,
	) -> (Self, CommitmentEventStream, impl Future<Output = Result<(), anyhow::Error>> + Send) {
		let (sender, receiver) = mpsc::channel(16);
		let (event_sender, mut event_receiver) = mpsc::channel(16);
		let event_stream = Box::pin(try_stream! {
			while let Some(event) = event_receiver.recv().await {
				yield event;
			}
		});
		let task = BackgroundTask { receiver, client, event_sender };
		(Self { sender }, event_stream, task.run())
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

impl<C: McrSettlementClientOperations> BackgroundTask<C> {
	async fn run(mut self) -> Result<(), anyhow::Error> {
		let mut settlement_stream = self.client.stream_block_commitments().await?;
		let mut max_height = self.client.get_max_tolerable_block_height().await?;
		let mut ahead_of_settlement = false;
		let mut commitments_to_settle = BTreeMap::new();
		let mut batch_acc = Vec::new();
		loop {
			tokio::select! {
				Some(block_commitment) = self.receiver.recv(), if !ahead_of_settlement => {
					println!("Received commitment: {:?}", block_commitment);
					commitments_to_settle.insert(
						block_commitment.height,
						block_commitment.commitment.clone(),
					);
					if block_commitment.height > max_height {
						ahead_of_settlement = true;
						let batch = mem::replace(&mut batch_acc, Vec::new());
						self.client.post_block_commitment_batch(batch).await?;
					}
					batch_acc.push(block_commitment);
				}
				Some(res) = settlement_stream.next() => {
					let settled_commitment = res?;
					println!("Received settlement: {:?}", settled_commitment);
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
						self.event_sender.send(event).await?;
					} else if let Some((&lh, _)) = commitments_to_settle.last_key_value() {
						if lh < height {
							// Settlement has left some commitments behind, but the client could
							// deliver them of order?
							todo!("Handle falling behind on settlement")
						}
					}
					// Remove back-pressure if we can proceed settling new blocks.
					if ahead_of_settlement {
						let new_max_height = self.client.get_max_tolerable_block_height().await?;
						if new_max_height > max_height {
							max_height = new_max_height;
							ahead_of_settlement = false;
						}
					}
				}
				else => break
			}
		}
		Ok(())
	}
}
