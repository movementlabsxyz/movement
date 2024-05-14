use crate::{CommitmentStream, McrSettlementClientOperations};
use movement_types::BlockCommitment;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub struct McrSettlementClient {
	commitments: Arc<RwLock<HashMap<u64, BlockCommitment>>>,
	stream_sender: mpsc::Sender<Result<BlockCommitment, anyhow::Error>>,
	// todo: this is logically dangerous, but it's just a stub
	stream_receiver: Arc<RwLock<mpsc::Receiver<Result<BlockCommitment, anyhow::Error>>>>,
    pub current_height: Arc<RwLock<u64>>,
    pub block_lead_tolerance: u64
}

impl McrSettlementClient {
	pub fn new() -> Self {
		let (stream_sender, receiver) = mpsc::channel(10);
		McrSettlementClient {
			commitments: Arc::new(RwLock::new(HashMap::new())),
			stream_sender,
			stream_receiver: Arc::new(RwLock::new(receiver)),
            current_height: Arc::new(RwLock::new(0)),
            block_lead_tolerance: 16,
		}
	}
}


#[async_trait::async_trait]
impl McrSettlementClientOperations for McrSettlementClient {
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {

        let height = block_commitment.height;

        {
            let mut commitments = self.commitments.write().await;
            commitments.insert(block_commitment.height, block_commitment.clone());
            self.stream_sender.send(Ok(block_commitment)).await?; // Simulate sending to the stream.
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
		let receiver = self.stream_receiver.clone();
		let stream = async_stream::try_stream! {
			let mut receiver = receiver.write().await;
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

		let client = McrSettlementClient::new();
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
        let client = McrSettlementClient::new();
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
