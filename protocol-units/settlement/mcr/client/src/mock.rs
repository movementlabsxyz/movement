use crate::{CommitmentStream, McrSettlementClientOperations};
use movement_types::BlockCommitment;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;

#[derive(Clone)]
pub struct MockMcrSettlementClient {
	commitments: Arc<RwLock<HashMap<u64, BlockCommitment>>>,
	stream_sender: mpsc::Sender<Result<BlockCommitment, anyhow::Error>>,
	stream_receiver: Arc<Mutex<Option<mpsc::Receiver<Result<BlockCommitment, anyhow::Error>>>>>,
    pub current_height: Arc<RwLock<u64>>,
    pub block_lead_tolerance: u64
}

impl MockMcrSettlementClient {
	pub fn new() -> Self {
		let (stream_sender, receiver) = mpsc::channel(10);
		MockMcrSettlementClient {
			commitments: Arc::new(RwLock::new(HashMap::new())),
			stream_sender,
			stream_receiver: Arc::new(Mutex::new(Some(receiver))),
            current_height: Arc::new(RwLock::new(0)),
            block_lead_tolerance: 16,
		}
	}

	/// Overrides the commitment to settle on at given height.
	///
	/// To have effect, this method needs to be called before a commitment is
	/// posted for this height with the `McrSettlementClientOperations` API.
	pub async fn settle(&self, commitment: BlockCommitment) {
		let mut commitments = self.commitments.write().await;
		commitments.insert(commitment.height, commitment);
	}
}

#[async_trait::async_trait]
impl McrSettlementClientOperations for MockMcrSettlementClient {
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {

        let height = block_commitment.height;

        {
            let mut commitments = self.commitments.write().await;
			let settled = commitments.entry(block_commitment.height).or_insert(block_commitment);
			// Simulate sending to the stream
            self.stream_sender.send(Ok(settled.clone())).await?;
        }

        {
            let mut current_height = self.current_height.write().await;
            if height > *current_height {
                *current_height = height;
            }
        }

		Ok(())
	}

    async fn post_block_commitment_batch(&self, block_commitment: Vec<BlockCommitment>) -> Result<(), anyhow::Error> {
        for commitment in block_commitment {
            self.post_block_commitment(commitment).await?;
        }
        Ok(())
    }

	async fn stream_block_commitments(&self) -> Result<CommitmentStream, anyhow::Error> {
		let receiver = self.stream_receiver
			.lock()
			.unwrap()
			.take()
			.expect("stream_block_commitments already called");
        Ok(Box::pin(ReceiverStream::new(receiver)))
    }

	async fn get_commitment_at_height(
		&self,
		height: u64,
	) -> Result<Option<BlockCommitment>, anyhow::Error> {
		let guard = self.commitments.write().await;
		Ok(guard.get(&height).cloned())
	}

    async fn get_max_tolerable_block_height(&self) -> Result<u64, anyhow::Error> {
        Ok(*self.current_height.read().await + self.block_lead_tolerance)
    }

}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_types::Commitment;
	use tokio_stream::StreamExt;

    #[tokio::test]
	async fn test_post_block_commitment() -> Result<(), anyhow::Error> {

		let client = MockMcrSettlementClient::new();
		let commitment = BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment::test(),
		};
		client.post_block_commitment(commitment.clone()).await.unwrap();
		let guard = client.commitments.write().await;
		assert_eq!(guard.get(&1), Some(&commitment));

        assert_eq!(*client.current_height.read().await, 1);
        assert_eq!(client.get_max_tolerable_block_height().await?, 17);

		Ok(())
	}

    #[tokio::test]
    async fn test_post_block_commitment_batch() -> Result<(), anyhow::Error> {
        let client = MockMcrSettlementClient::new();
        let commitment = BlockCommitment {
            height: 1,
            block_id: Default::default(),
            commitment: Commitment::test(),
        };
        let commitment2 = BlockCommitment {
            height: 2,
            block_id: Default::default(),
            commitment: Commitment::test(),
        };
        client.post_block_commitment_batch(vec![
            commitment.clone(),
            commitment2.clone(),
        ]).await.unwrap();
        let guard = client.commitments.write().await;
        assert_eq!(guard.get(&1), Some(&commitment));
        assert_eq!(guard.get(&2), Some(&commitment2));
        Ok(())
    }
	
	#[tokio::test]
	async fn test_stream_block_commitments() -> Result<(), anyhow::Error> {
		let client = MockMcrSettlementClient::new();
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

	#[tokio::test]
	async fn test_override_block_commitments() -> Result<(), anyhow::Error> {
		let client = MockMcrSettlementClient::new();
		let commitment = BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment::test(),
		};
		client.settle(commitment.clone()).await;
		client.post_block_commitment(BlockCommitment {
			height: 1,
			block_id: Default::default(),
			commitment: Commitment([1; 32]),
		}).await.unwrap();
		let mut stream = client.stream_block_commitments().await?;
		assert_eq!(stream.next().await.unwrap().unwrap(), commitment);
		Ok(())
	}
}
