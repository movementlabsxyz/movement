use movement_types::block::BlockCommitment;
use tokio_stream::Stream;
pub mod mock;

// FIXME: mock exports
// #[cfg(feature = "mock")]
// pub use mock::*;

pub mod eth_client;

#[cfg(feature = "eth")]
pub use eth_client::McrSettlementClient;

pub mod send_eth_transaction;

type CommitmentStream =
	std::pin::Pin<Box<dyn Stream<Item = Result<BlockCommitment, anyhow::Error>> + Send>>;

#[async_trait::async_trait]
pub trait McrSettlementClientOperations {
	/// Posts a block commitment to the settlement client.
	async fn post_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error>;

	/// Posts a batch of block commitments to the settlement client.
	async fn post_block_commitment_batch(
		&self,
		block_commitment: Vec<BlockCommitment>,
	) -> Result<(), anyhow::Error>;

	/// Forces a block commitment
	/// This will only work in admin mode
	async fn force_block_commitment(
		&self,
		block_commitment: BlockCommitment,
	) -> Result<(), anyhow::Error>;

	/// Streams block commitments from the settlement client.
	async fn stream_block_commitments(&self) -> Result<CommitmentStream, anyhow::Error>;

	/// Gets the accepted commitment at the given height.
	async fn get_commitment_at_height(
		&self,
		height: u64,
	) -> Result<Option<BlockCommitment>, anyhow::Error>;

	/// Gets the commitment this validator has made at a given height
	async fn get_posted_commitment_at_height(
		&self,
		height: u64,
	) -> Result<Option<BlockCommitment>, anyhow::Error>;

	/// Gets the max tolerable block height.
	async fn get_max_tolerable_block_height(&self) -> Result<u64, anyhow::Error>;
}
