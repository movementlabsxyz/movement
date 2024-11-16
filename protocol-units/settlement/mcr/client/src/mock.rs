use crate::{CommitmentStream, McrSettlementClientOperations};
use mcr_settlement_config::Config;
use movement_types::block::BlockCommitment;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

#[derive(Clone)]
pub struct McrSettlementClient {
	commitments: Arc<RwLock<BTreeMap<u64, BlockCommitment>>>,
	stream_sender: mpsc::Sender<Result<BlockCommitment, anyhow::Error>>,
	stream_receiver: Arc<Mutex<Option<mpsc::Receiver<Result<BlockCommitment, anyhow::Error>>>>>,
	pub current_height: Arc<RwLock<u64>>,
	pub block_lead_tolerance: u64,
	paused_at_height: Arc<RwLock<Option<u64>>>,
}

impl McrSettlementClient {
	pub fn new() -> Self {
		let (stream_sender, receiver) = mpsc::channel(10);
		McrSettlementClient {
			commitments: Arc::new(RwLock::new(BTreeMap::new())),
			stream_sender,
			stream_receiver: Arc::new(Mutex::new(Some(receiver))),
			current_height: Arc::new(RwLock::new(0)),
			block_lead_tolerance: 16,
			paused_at_height: Arc::new(RwLock::new(None)),
		}
	}

	pub async fn build_with_config(_config: &Config) -> Result<Self, anyhow::Error> {
		info!("Building with config.");
		Ok(Self::new())
	}

	/// Overrides the commitment to settle on at given height.
	///
	/// To have effect, this method needs to be called before a commitment is
	/// posted for this height with the `McrSettlementClientOperations` API.
	pub async fn override_block_commitment(&self, commitment: BlockCommitment) {
		let mut commitments = self.commitments.write().await;
		commitments.insert(commitment.height(), commitment);
	}

	/// Stop streaming commitments after the given height.
	///
	/// Any posted commitments will be accumulated.
	pub async fn pause_after(&self, height: u64) {
		let mut paused_at_height = self.paused_at_height.write().await;
		*paused_at_height = Some(height);
	}

	/// Stream any commitments that have been posted following the height
	/// at which `pause` was called, and resume streaming any newly posted
	/// commitments
	pub async fn resume(&self) {
		let resume_height = {
			let mut paused_at_height = self.paused_at_height.write().await;
			paused_at_height.take().expect("not paused")
		};
		{
			let commitments = self.commitments.read().await;
			for (_, commitment) in commitments.range(resume_height + 1..) {
				println!("resume sends commitment for height {}", commitment.height());
				self.stream_sender.send(Ok(commitment.clone())).await.unwrap();
			}
		}
	}
}

#[async_trait::async_trait]
impl McrSettlementClientOperations for McrSettlementClient {
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {
		let height = block_commitment.height();

		let settled = {
			let mut commitments = self.commitments.write().await;
			commitments.entry(block_commitment.height()).or_insert(block_commitment).clone()
		};
		{
			let paused_at_height = self.paused_at_height.read().await;
			match *paused_at_height {
				Some(ph) if ph < height => {}
				_ => {
					self.stream_sender.send(Ok(settled)).await?;
				}
			}
		}

		{
			let mut current_height = self.current_height.write().await;
			if height > *current_height {
				*current_height = height;
			}
		}

		Ok(())
	}

	async fn post_block_commitment_batch(
		&self,
		block_commitment: Vec<BlockCommitment>,
	) -> Result<(), anyhow::Error> {
		for commitment in block_commitment {
			self.post_block_commitment(commitment).await?;
		}
		Ok(())
	}

	async fn force_block_commitment(
		&self,
		_block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error> {
		unimplemented!()
	}

	async fn stream_block_commitments(&self) -> Result<CommitmentStream, anyhow::Error> {
		let receiver = self
			.stream_receiver
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
		let guard = self.commitments.read().await;
		Ok(guard.get(&height).cloned())
	}

	async fn get_max_tolerable_block_height(&self) -> Result<u64, anyhow::Error> {
		Ok(*self.current_height.read().await + self.block_lead_tolerance)
	}
}

#[cfg(test)]
pub mod test {

	use super::*;
	use movement_types::block::Commitment;

	use futures::future;
	use tokio::select;
	use tokio_stream::StreamExt;

	#[tokio::test]
	async fn test_post_block_commitment() -> Result<(), anyhow::Error> {
		let client = McrSettlementClient::new();
		let commitment = BlockCommitment::new(1, Default::default(), Commitment::test());
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
		let commitment = BlockCommitment::new(1, Default::default(), Commitment::test());
		let commitment2 = BlockCommitment::new(1, Default::default(), Commitment::test());
		client
			.post_block_commitment_batch(vec![commitment.clone(), commitment2.clone()])
			.await
			.unwrap();
		let guard = client.commitments.write().await;
		assert_eq!(guard.get(&1), Some(&commitment));
		assert_eq!(guard.get(&2), Some(&commitment2));
		Ok(())
	}

	#[tokio::test]
	async fn test_stream_block_commitments() -> Result<(), anyhow::Error> {
		let client = McrSettlementClient::new();
		let commitment = BlockCommitment::new(1, Default::default(), Commitment::test());
		client.post_block_commitment(commitment.clone()).await.unwrap();
		let mut stream = client.stream_block_commitments().await?;
		assert_eq!(stream.next().await.unwrap().unwrap(), commitment);
		Ok(())
	}

	#[tokio::test]
	async fn test_override_block_commitments() -> Result<(), anyhow::Error> {
		let client = McrSettlementClient::new();
		let commitment = BlockCommitment::new(2, Default::default(), Commitment::test());
		client.override_block_commitment(commitment.clone()).await;
		client
			.post_block_commitment(BlockCommitment::new(2, Default::default(), Commitment::test()))
			.await
			.unwrap();
		let mut stream = client.stream_block_commitments().await?;
		assert_eq!(stream.next().await.expect("stream has ended")?, commitment);
		Ok(())
	}

	#[tokio::test]
	async fn test_pause() -> Result<(), anyhow::Error> {
		let client = McrSettlementClient::new();
		let commitment = BlockCommitment::new(2, Default::default(), Commitment::test());
		client.pause_after(1).await;
		client.post_block_commitment(commitment.clone()).await?;
		let commitment2 = BlockCommitment::new(2, Default::default(), Commitment::test());
		client.post_block_commitment(commitment2).await?;
		let mut stream = client.stream_block_commitments().await?;
		assert_eq!(stream.next().await.expect("stream has ended")?, commitment);
		select! {
			biased;
			_ = stream.next() => panic!("stream should be paused"),
			_ = future::ready(()) => {}
		}
		Ok(())
	}

	#[tokio::test]
	async fn test_resume() -> Result<(), anyhow::Error> {
		let client = McrSettlementClient::new();
		let commitment = BlockCommitment::new(2, Default::default(), Commitment::test());
		client.pause_after(1).await;
		client.post_block_commitment(commitment.clone()).await?;
		let commitment2 = BlockCommitment::new(2, Default::default(), Commitment::test());
		client.post_block_commitment(commitment2.clone()).await?;
		let mut stream = client.stream_block_commitments().await?;
		assert_eq!(stream.next().await.expect("stream has ended")?, commitment);
		client.resume().await;
		assert_eq!(stream.next().await.expect("stream has ended")?, commitment2);
		Ok(())
	}
}
