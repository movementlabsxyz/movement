use crate::{CommitmentStream, McrSettlementClientOperations};
use movement_types::BlockCommitment;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub struct McrSettlementClient {
	commitments: Arc<Mutex<HashMap<u64, BlockCommitment>>>,
	stream_sender: mpsc::Sender<Result<BlockCommitment, anyhow::Error>>,
	// todo: this is logically dangerous, but it's just a stub
	stream_receiver: Arc<Mutex<mpsc::Receiver<Result<BlockCommitment, anyhow::Error>>>>,
}

impl McrSettlementClient {
	pub fn new() -> Self {
		let (stream_sender, receiver) = mpsc::channel(10);
		McrSettlementClient {
			commitments: Arc::new(Mutex::new(HashMap::new())),
			stream_sender,
			stream_receiver: Arc::new(Mutex::new(receiver)),
		}
	}
}

#[tonic::async_trait]
impl McrSettlementClientOperations for McrSettlementClient {
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {
		let mut commitments = self.commitments.lock().await;
		commitments.insert(block_commitment.height, block_commitment.clone());
		self.stream_sender.send(Ok(block_commitment)).await?; // Simulate sending to the stream.
		Ok(())
	}

	async fn stream_block_commitments(&self) -> Result<CommitmentStream, anyhow::Error> {
		let receiver = self.stream_receiver.clone();
		let stream = async_stream::try_stream! {
			let mut receiver = receiver.lock().await;
			while let Some(commitment) = receiver.recv().await {
				yield commitment?;
			}
		};

		Ok(Box::pin(stream) as CommitmentStream)
	}

	async fn get_commitment_at_height(
		&self,
		height: u64,
	) -> Result<Option<BlockCommitment>, anyhow::Error> {
		let guard = self.commitments.lock().await;
		Ok(guard.get(&height).cloned())
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_types::Commitment;
	use tokio_stream::StreamExt;

	#[tokio::test]
	async fn test_post_block_commitment() -> Result<(), anyhow::Error> {
		let client = McrSettlementClient::new();
		let commitment = BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment::test(),
		};
		client.post_block_commitment(commitment.clone()).await.unwrap();
		let guard = client.commitments.lock().await;
		assert_eq!(guard.get(&1), Some(&commitment));
		Ok(())
	}

	#[tokio::test]
	async fn test_stream_block_commitments() -> Result<(), anyhow::Error> {
		let client = McrSettlementClient::new();
		let commitment = BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment::test(),
		};
		client.post_block_commitment(commitment.clone()).await.unwrap();
		let mut stream = client.stream_block_commitments().await?;
		assert_eq!(stream.next().await.unwrap().unwrap(), commitment);
		Ok(())
	}
}
